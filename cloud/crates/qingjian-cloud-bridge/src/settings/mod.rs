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
use crate::memory::MemoryError;

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

    /// 云联想（`[predict] enabled`，与 Mac 同一个开关）；iOS 上固定走青简 Cloud 的代理。
    pub cloud_prediction: bool,

    pub domains: Vec<DomainSetting>,

    pub phrases: Vec<CustomPhrase>,

    /// 改写用的默认技能（技能包 id）；某个人身上指定了就用他的。旧配置文件没有这一项时按 `polish`。
    #[serde(default = "default_rewrite_skill")]
    pub rewrite_skill: String,
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
            cloud_prediction: config.predict.enabled,
            domains,
            phrases: config.custom_phrases,
            rewrite_skill: rewrite_skill(config_path),
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
        set_bool("predict", "enabled", settings.cloud_prediction)?;
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
        Config::set_value(
            config_path,
            "rewrite",
            "skill",
            settings.rewrite_skill.as_str(),
        )
        .map_err(|e| e.to_string())?;
        Config::set_custom_phrases(config_path, &settings.phrases)
    }
}

fn default_rewrite_skill() -> String {
    crate::rewrite::DEFAULT_SKILL_ID.to_owned()
}

/// 改写用的默认技能（`[rewrite] skill`）：只有 iOS 用，Mac 的 [`Config`] 按分节读，多出来的分节与键都忽略。
/// 文件不在、读不了、这一项没写或写得不是字符串时都按缺省（[`crate::rewrite::DEFAULT_SKILL_ID`]）；
/// 值本身合不合法（[`crate::rewrite::is_skill_id`]）由调用方判，认不得的技能由键盘那边回退。
pub fn rewrite_skill(config_path: &Path) -> String {
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|source| source.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|document| {
            document
                .get("rewrite")?
                .get("skill")?
                .as_str()
                .map(str::to_owned)
        })
        .unwrap_or_else(default_rewrite_skill)
}

/// 把 `[rewrite] skill` 改成 `skill`，文件里别的内容、注释与顺序原样保留（整节不存在时只加这一节）。
/// 文件不在时从空文档起步，只写这一项——其余设置由设置页或 Mac 同步补上。
///
/// 只校验 id 的形状（[`crate::rewrite::is_skill_id`]），不校验这个技能现在在不在：技能包随版本增删，
/// 写进来一个暂时认不得的 id 由壳按当前技能列表回退。
pub fn set_rewrite_skill(config_path: &Path, skill: &str) -> Result<(), MemoryError> {
    if !crate::rewrite::is_skill_id(skill) {
        return Err(MemoryError::Invalid("技能编号不对"));
    }
    // 读-改-写原始 TOML，与上面的 [`rewrite_skill`] 对称：`Config::set_value` 也能写这个键，
    // 但它给不存在的文件铺的是一整套模板，这里只加这一项（其余设置由设置页或 Mac 同步补上）
    let source = match std::fs::read_to_string(config_path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(MemoryError::Io(error)),
    };
    let mut document: toml_edit::DocumentMut = source
        .parse()
        .map_err(|_| MemoryError::Invalid("设置文件读不了，没有改动"))?;
    // 分节不存在时先建成标准表，否则 toml_edit 会写成顶层的行内表 `rewrite = { skill = "…" }`
    if !document
        .get("rewrite")
        .is_some_and(toml_edit::Item::is_table)
    {
        document["rewrite"] = toml_edit::table();
    }
    document["rewrite"]["skill"] = toml_edit::value(skill);
    // 写临时文件再改名：键盘随时可能被杀，不能留半个配置文件
    crate::cloud_config::write_atomic(config_path, document.to_string().as_bytes(), true)
        .map_err(MemoryError::Io)
}

fn domain_label(path: &Path) -> Option<String> {
    let dictionary = Dictionary::from_path(path).ok()?;
    let name = &dictionary.metadata()?.name;
    Some(name.strip_prefix(DOMAIN_PREFIX).unwrap_or(name).to_owned())
}
