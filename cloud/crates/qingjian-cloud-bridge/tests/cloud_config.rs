//! `cloud.toml` 的读取：没填全、格式不对都当离线。

use qingjian_cloud_bridge::CloudConfig;

fn load(text: &str) -> Option<CloudConfig> {
    let path = std::env::temp_dir().join(format!(
        "qj-cloud-{}-{}.toml",
        std::process::id(),
        text.len()
    ));
    std::fs::write(&path, text).unwrap();
    let config = CloudConfig::load(&path);
    std::fs::remove_file(&path).ok();
    config
}

#[test]
fn full_config_turns_everything_on() {
    let config = load("server = \"https://pinyin.synon.ai/\"\ntoken = \"t\"\n").unwrap();
    assert!(config.llm && config.sync);
    assert_eq!(config.llm_base_url(), "https://pinyin.synon.ai/v1");
}

#[test]
fn switches_are_respected() {
    let config = load("server = \"s\"\ntoken = \"t\"\nllm = false\n").unwrap();
    assert!(!config.llm && config.sync);
}

#[test]
fn missing_token_or_bad_toml_means_offline() {
    assert!(load("server = \"s\"\n").is_none());
    assert!(load("server = \"s\"\ntoken = \"  \"\n").is_none());
    assert!(load("server = ").is_none());
    assert!(CloudConfig::load(std::path::Path::new("/nonexistent/cloud.toml")).is_none());
}
