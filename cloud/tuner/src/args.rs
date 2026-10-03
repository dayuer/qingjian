//! 命令行参数，都可以用环境变量给（Docker 里用环境变量）。

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "qingjian-cloud-tuner", version, about = "青简 Cloud 纠错闭环")]
pub struct Args {
    /// 服务器地址。
    #[arg(long, env = "QINGJIAN_CLOUD_SERVER")]
    pub server: String,

    /// tuner 这台「设备」的令牌（服务器上 `device add tuner`）。
    #[arg(long, env = "QINGJIAN_TUNER_TOKEN", hide_env_values = true)]
    pub token: String,

    /// 上游的 qingjian-cli。
    #[arg(long, env = "QINGJIAN_CLI", default_value = "qingjian-cli")]
    pub cli: PathBuf,

    /// 产品数据目录（含 dict.qj、lm.qj、dicts/），路径要以 `data/generated` 结尾。
    #[arg(
        long,
        env = "QINGJIAN_DATA",
        default_value = "/opt/qingjian/data/generated"
    )]
    pub data: PathBuf,

    /// 状态与报告放哪。
    #[arg(long, env = "QINGJIAN_TUNER_STATE", default_value = "/data")]
    pub state: PathBuf,

    /// 问哪个模型（服务器配了 QINGJIAN_LLM_MODEL 时以服务器的为准）。
    #[arg(
        long,
        env = "QINGJIAN_TUNER_MODEL",
        default_value = "deepseek-v4-flash"
    )]
    pub model: String,

    /// 只出报告，不推送。
    #[arg(long, env = "QINGJIAN_TUNER_DRY_RUN")]
    pub dry_run: bool,

    /// 每隔多少小时跑一次；0 为只跑一次就退出。
    #[arg(long, env = "QINGJIAN_TUNER_EVERY_HOURS", default_value_t = 0)]
    pub every_hours: u64,

    /// 每轮最多体检多少个用户词。
    #[arg(long, default_value_t = 300)]
    pub max_audit: usize,

    /// 每轮最多看多少个分次选完的词。
    #[arg(long, default_value_t = 300)]
    pub max_compositions: usize,

    /// 每轮话题补词最多几个；0 为不做。
    #[arg(long, default_value_t = 30)]
    pub max_topics: usize,

    /// 前多少比例的上屏用来找问题，其余留出来回放把关。
    #[arg(long, default_value_t = 0.8)]
    pub train_ratio: f64,

    /// 留出段至少多少次上屏才做回放门槛；不够时只应用有用户实际证据的修正（不做话题补词）。
    #[arg(long, default_value_t = 30)]
    pub min_holdout: usize,
}
