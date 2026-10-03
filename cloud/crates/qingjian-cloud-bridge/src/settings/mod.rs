//! iOS 主 App 的设置页：从与 Mac 同一格式的 `config.toml` 里读出 iOS 键盘用得上的那些设置，改了逐项写回。
//! 读写都走 `qingjian-platform`（Mac 也用它），文件里 Mac 专属的设置与注释原样保留；这份文件经青简 Cloud 与 Mac 同步。

mod domain;
mod scheme_option;

use std::path::Path;

use qingjian_core::{CustomPhrase, FuzzyRules, ShuangpinScheme};
use qingjian_dictionary::Dictionary;
use qingjian_platform::{Config, Scheme};
use serde::{Deserialize, Serialize};

pub use self::domain::DomainSetting;
pub use self::scheme_option::SchemeOption;

/// 领域词库元数据名字里统一的前缀。
const DOMAIN_PREFIX: &str = "青简领域词库：";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// 当前拼音方案的 key（`pinyin` / `xiaohe` …）。
    pub scheme: String,

    /// 可选的方案（只读，写回时忽略）。
    #[serde(default)]
    pub schemes: Vec<SchemeOption>,

    pub fuzzy: FuzzyRules,

    pub traditional: bool,

    pub full_width_punctuation: bool,

    pub learning: bool,

    pub domains: Vec<DomainSetting>,

    pub phrases: Vec<CustomPhrase>,
}

impl Settings {
    /// 读 `config_path`（没有就是缺省设置），领域词库按 `dicts_dir` 里随包的文件列。
    pub fn read(config_path: &Path, dicts_dir: &Path) -> Self {
        let config = Config::load(config_path).unwrap_or_else(|error| {
            tracing::warn!(%error, "config.toml 读不了，按缺省显示");
            Config::default()
        });
        let scheme = match config.general.scheme() {
            Scheme::Shuangpin(scheme) => Scheme::Shuangpin(scheme),
            _ => Scheme::Pinyin,
        };
        let mut domains: Vec<DomainSetting> =
            qingjian_platform::extra_dictionaries::list(dicts_dir)
                .into_iter()
                .map(|(id, path)| DomainSetting {
                    enabled: config.dictionaries.is_domain_enabled(&id),
                    label: domain_label(&path).unwrap_or_else(|| id.clone()),
                    id,
                })
                .collect();
        domains.sort_by(|a, b| a.id.cmp(&b.id));
        Self {
            scheme: scheme.key().to_owned(),
            schemes: std::iter::once(Scheme::Pinyin)
                .chain(ShuangpinScheme::ALL.into_iter().map(Scheme::Shuangpin))
                .map(|scheme| SchemeOption {
                    key: scheme.key().to_owned(),
                    label: scheme.label().to_owned(),
                })
                .collect(),
            fuzzy: config.fuzzy,
            traditional: config.general.traditional,
            full_width_punctuation: config.general.full_width_punctuation,
            learning: config.general.learning,
            domains,
            phrases: config.custom_phrases,
        }
    }

    /// 把设置逐项写回 `config_path`（没有就从模板建）；自定义短语不合法时整份不写、返回原因。
    pub fn write(&self, config_path: &Path) -> Result<(), String> {
        let settings = self;
        qingjian_core::custom_phrase::validate_phrases(&settings.phrases)?;
        let set_bool = |section: &str, key: &str, value: bool| {
            Config::set_bool(config_path, section, key, value).map_err(|e| e.to_string())
        };
        Config::set_value(config_path, "general", "scheme", settings.scheme.as_str())
            .map_err(|e| e.to_string())?;
        set_bool("general", "traditional", settings.traditional)?;
        set_bool(
            "general",
            "full_width_punctuation",
            settings.full_width_punctuation,
        )?;
        set_bool("general", "learning", settings.learning)?;
        let fuzzy = serde_json::to_value(settings.fuzzy).map_err(|e| e.to_string())?;
        if let Some(rules) = fuzzy.as_object() {
            for (key, value) in rules {
                set_bool("fuzzy", key, value.as_bool().unwrap_or(false))?;
            }
        }
        let domains: toml_edit::Array = settings
            .domains
            .iter()
            .filter(|d| d.enabled)
            .map(|d| d.id.as_str())
            .collect();
        Config::set_value(config_path, "dictionaries", "domains", domains)
            .map_err(|e| e.to_string())?;
        Config::set_custom_phrases(config_path, &settings.phrases)
    }
}

fn domain_label(path: &Path) -> Option<String> {
    let dictionary = Dictionary::from_path(path).ok()?;
    let name = &dictionary.metadata()?.name;
    Some(name.strip_prefix(DOMAIN_PREFIX).unwrap_or(name).to_owned())
}
