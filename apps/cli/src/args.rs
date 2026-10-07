use std::path::PathBuf;

use clap::Parser;

/// 按优先级挑一个存在的数据文件：`data/generated/` 里打包好的 `.qj`、那里的 TSV、仓库自带的产品数据
/// （`assets/lexicon/dict.tsv`、`assets/glossary/glossary-*.tsv`、`assets/lexicon/english.tsv`），最后是 `assets/sample/` 的样例。
pub fn default_data_file(name: &str) -> PathBuf {
    let generated = PathBuf::from("data/generated").join(name);
    let packed = generated.with_extension("qj");
    let shipped = if name.starts_with("glossary-") {
        PathBuf::from("assets/glossary").join(name)
    } else {
        PathBuf::from("assets/lexicon").join(name)
    };
    for candidate in [packed, generated, shipped] {
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("assets/sample").join(name)
}

/// 缺省配置文件位置：与输入法共用同一份。
pub fn default_config_file() -> PathBuf {
    if cfg!(target_os = "macos")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join("Library/Application Support/Qingjian/config.toml");
    }
    PathBuf::from("config.toml")
}

#[derive(Debug, Parser)]
#[command(name = "qingjian", about = "青简输入法 Core 测试工具")]
pub struct Args {
    /// 词库路径（TSV）。缺省：data/generated/dict.tsv 存在就用它，否则 assets/sample/dict.tsv
    #[arg(long)]
    pub dict: Option<PathBuf>,

    /// 释义表路径。缺省：data/generated/glossary-<language>.tsv 存在就用它，否则 assets/sample/ 下的同名文件
    #[arg(long)]
    pub glossary: Option<PathBuf>,

    /// 学习语言：en / ja / es。也可用环境变量 QINGJIAN_LEARNING_LANGUAGE
    #[arg(long, env = "QINGJIAN_LEARNING_LANGUAGE", default_value = "en")]
    pub language: String,

    /// 附加词库（.qj 或 TSV），可给多个，与主词库一起查
    #[arg(long)]
    pub extra_dict: Vec<PathBuf>,

    /// 辅码码表（.qj，或 `词<TAB>码` 的 TSV），可给多个一起筛。给了之后 `kaifa;kf` 这样的输入
    /// 按辅码态走：触发键进辅码态、之后的字母按码缩小候选
    #[arg(long)]
    pub aux_table: Vec<PathBuf>,

    /// 查码：打印这些词在已装码表里的全部码（配 --aux-table 用），逗号分隔或多次给；查完即退出
    #[arg(long, value_delimiter = ',')]
    pub aux_query: Vec<String>,

    /// 英文词表路径（中英混输）。缺省：data/generated/english.tsv 存在就用它，否则不启用
    #[arg(long)]
    pub english: Option<PathBuf>,

    /// 用户词频文件；给了就在退出时写回，不给则只在本次会话内学习
    #[arg(long)]
    pub user_dict: Option<PathBuf>,

    /// 配置文件路径。缺省：~/Library/Application Support/Qingjian/config.toml（macOS）或 ./config.toml
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// 启用云联想（无视配置里的 enabled）；密钥来自配置或 QINGJIAN_API_KEY（`api_key_env`）
    #[arg(long)]
    pub predict: bool,

    /// 模糊音，逗号分隔（z-zh,c-ch,s-sh,n-l,f-h,l-r,an-ang,en-eng,in-ing），`all` 全开；给了就覆盖配置里的 [fuzzy]
    #[arg(long, value_delimiter = ',')]
    pub fuzzy: Vec<String>,

    /// 英文模式（输入法里是 Caps Lock 亮着）：字母不当拼音，候选来自英文词表的补全与拼错纠正
    #[arg(long)]
    pub english_mode: bool,

    /// 打开中文优先（配置 [general] chinese_first = true）：整段是英文词时中文候选排第一、英文第二，评测两种排法用
    #[arg(long)]
    pub chinese_first: bool,

    /// 双拼方案（xiaohe / ziranma / microsoft / sogou / abc / xiaolang / shoudao），覆盖配置里的 [general] shuangpin；off 强制全拼
    #[arg(long)]
    pub shuangpin: Option<String>,

    /// 形码码表（五笔）的 TSV 文件（`词\t编码\t词频`）：给了就用编码查表，不走拼音那一套
    #[arg(long, value_name = "码表")]
    pub wubi: Option<PathBuf>,

    /// 神经重打分：字级 Transformer 的 .qjm 文件或导出目录（model.safetensors / config.json / vocab.json），整句前几条路径用它重排
    #[arg(long)]
    pub neural: Option<PathBuf>,

    /// 神经重打分的权重 λ（0 到 1，缺省 0.5）：最终分 = 路径分 + λ·(神经分 − 静态二元分)，个人学习与代价不受影响
    #[arg(long)]
    pub neural_weight: Option<f64>,

    /// 神经重打分的门槛（nat，缺省不设）：路径分落后最优路径超过这么多的不参与重排
    #[arg(long)]
    pub neural_margin: Option<f64>,

    /// 神经重打分给模型看的前文字符数（缺省 64，0 为不给前文）
    #[arg(long)]
    pub neural_context: Option<usize>,

    /// 神经重打分走后台线程（输入法壳里的接法）：查询先按词级模型出候选，再请求 / 等待重打分后重查一次；结果应与同步一致
    #[arg(long)]
    pub neural_async: bool,

    /// 知微（含章·知微，字级模型）的 .qjm 或导出目录：给第一页词级候选按前文打分重排（素笺）；
    /// 与 --neural-async 同用时走后台线程（壳里的接法）
    #[arg(long)]
    pub word_model: Option<PathBuf>,

    /// 词级重排里神经分的权重 λ_w（0 到 1，缺省 0.5）
    #[arg(long, requires = "word_model")]
    pub word_weight: Option<f64>,

    /// 逐键模式：把每个输入当作一键一键敲进去，每个前缀都查一次，打印每键各阶段耗时（性能测试用）
    #[arg(long)]
    pub typing: bool,

    /// 只显示前 N 个候选
    #[arg(long, default_value_t = 9)]
    pub limit: usize,

    /// 回放评测：读输入日志（input-log.jsonl），把每次上屏时的键重新喂给引擎，算首选命中率等指标。只在内存里学习，不写任何文件
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// 同音快照：按无调音节串分组、每组前 5 名写成这个文件（进仓库）。与 `--check` 一起用时只比对不改写
    #[arg(long)]
    pub homophone_snapshot: Option<PathBuf>,

    /// 与 `--homophone-snapshot` 指的快照比对，不一致就失败（CI 用）
    #[arg(long, requires = "homophone_snapshot")]
    pub check: bool,

    /// 回放 / 整句评测时打印前 N 条没命中首选的例子
    #[arg(long, default_value_t = 20)]
    pub misses: usize,

    /// 覆盖引擎里的调参常数，`名=值`，逗号分隔或多次给。名字：lambda / k / cap / discount（个人 n-gram 插值 λ / K / 封顶 / 三元折扣），
    /// transpose / substitute / extra / missing / typo-cap / correction（敲错四类代价 / 个人折扣上限 / 整段纠错代价），
    /// choice（素笺：同输入串下选过的加分系数 β）
    #[arg(long, value_delimiter = ',')]
    pub tune: Vec<String>,

    /// 整句评测：读中文文本（一行一段，按标点切句、按词库转成全拼）或 `--eval-save` 冻结下来的三列文件，
    /// 冷启动喂给引擎看整句能不能还原原句；可给多个文件
    #[arg(long, num_args = 1..)]
    pub eval_text: Vec<PathBuf>,

    /// 把整句评测用到的句子集写成 `句子\t拼音\t上文` 三列文件，下次直接 `--eval-text` 它，保证比的是同一份句子
    #[arg(long)]
    pub eval_save: Option<PathBuf>,

    /// 语音转录纠错原型：读识别结果（一行一句，可带 `<Tab>参考文本`），按 `--terms` 的术语表
    /// 把同音、近音片段换回术语，报替换明细；带参考时报字错率与术语命中的前后对比
    #[arg(long, num_args = 1.., requires = "terms")]
    pub asr_fix: Vec<PathBuf>,

    /// 术语表：一行一条，也认逗号、顿号分隔（VoiceBridge 词库导出的格式）；
    /// 或 `错<Tab>对[<Tab>会议[<Tab>状态]]` 的纠错对，按词库读音提炼成术语
    #[arg(long, requires = "asr_fix")]
    pub terms: Option<PathBuf>,

    /// 纠错原型的宽松模式：识别结果里的片段本身是词库词也换
    #[arg(long, requires = "asr_fix")]
    pub asr_fix_loose: bool,

    /// 纠错原型留一场会议评测：转录第三列是会议、术语表第三列是学自哪场会议时，只学自本场的术语不纠本场
    #[arg(long, requires = "asr_fix")]
    pub asr_fix_holdout: bool,

    /// 纠错原型只用确认过的纠错对（术语表第四列为 candidate 的不用）
    #[arg(long, requires = "asr_fix")]
    pub asr_fix_confirmed: bool,

    /// 实验专用：P2C 按完整拼音给整句路径打分；仅用于 --eval-text，不看上文
    #[arg(long, requires = "eval_text", conflicts_with_all = ["neural", "neural_async", "neural_context"])]
    pub eval_p2c: Option<PathBuf>,

    /// 实验专用：在同一份句子集上让 P2C 直接生成整句（不经词图、不经重排），量重排架构的上限代价
    #[arg(long, requires = "eval_text")]
    pub eval_generate: Option<PathBuf>,

    /// 将整句评测逐条结果写成 JSONL（含句子、候选和延迟）
    #[arg(long, requires = "eval_text")]
    pub eval_details: Option<PathBuf>,

    /// 将回放逐条结果写成 JSONL（日志行号、来源、作用域、当时选的词、现在的名次与前五），按行号切前后两半或比两份词库用
    #[arg(long, requires = "replay")]
    pub replay_details: Option<PathBuf>,

    /// 回放另报干净口径：剔除撤销（R1）、上屏后删掉重打（R2）、汉字夹短字母残段（R3）的上屏再算，并按前后两半各报一次。规则见 `replay/clean`
    #[arg(long, requires = "replay")]
    pub clean: bool,

    /// 干净口径 2：`--clean` 的剔除规则，加上回放遇到撤销时像真实使用那样先逐字退格、让引擎回滚那次的学习。规则见 `replay/clean`
    #[arg(long, requires = "replay")]
    pub clean2: bool,

    /// 同拼音不同上文评测：读 `拼音\t前文\t期望` 三列（cloud/data/eval/context-pairs.tsv），
    /// 每对先给前文、再不给前文各查一次，报告两种设置下的首选命中率与按键同步部分的 p50 / p99
    #[arg(long)]
    pub eval_context: Option<PathBuf>,

    /// 把 --eval-context 的逐对结果写成 JSONL（拒绝覆盖）
    #[arg(long, requires = "eval_context")]
    pub eval_context_details: Option<PathBuf>,

    /// 本地续写评测：在这些文本（一行一段）上每 20 字取一个位置，前文给知微续写、下文当真值；--neural 要指向知微
    #[arg(long, num_args = 1.., requires = "neural")]
    pub eval_continuation: Vec<PathBuf>,

    /// 冷启动字词评测：读取 JSONL，不加载个人配置或个人学习文件
    #[arg(long, requires = "cold_output", conflicts_with_all = ["user_dict", "config", "predict", "replay", "eval_text", "english_mode", "shuangpin", "wubi", "aux_table", "tune", "fuzzy", "inputs", "neural_async"])]
    pub eval_cold: Option<PathBuf>,

    /// 冷启动逐词结果 JSONL（拒绝覆盖）
    #[arg(long, requires = "eval_cold")]
    pub cold_output: Option<PathBuf>,

    /// 同时评测 P2C 生成及联合候选；不改变引擎原候选
    #[arg(long, requires = "eval_cold")]
    pub cold_model: Option<PathBuf>,

    /// 直接查询这些拼音后退出；不给则进入交互模式
    pub inputs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 裸拼音是位置参数、落在 `inputs` 上。52ffa83 那次给 `--homophone-snapshot` 插参数时，
    /// 把 `replay` 的 `#[arg(long)]` 抢了过去，`replay` 变成位置参数，`qingjian-cli -- nihao`
    /// （CLAUDE.md 里的主力验证用法）被当成输入日志路径去读 —— 这条守住它。
    #[test]
    fn a_bare_argument_is_an_input_not_a_log_path() {
        let args = Args::parse_from(["qingjian", "nihao", "zaijian"]);
        assert_eq!(args.inputs, ["nihao", "zaijian"]);
        assert!(args.replay.is_none(), "裸拼音不该落进 replay：{args:?}");
    }

    /// 三个 flag 各归各的字段，谁也别抢谁的。
    #[test]
    fn long_flags_land_on_their_own_fields() {
        let args = Args::parse_from([
            "qingjian",
            "--replay",
            "log.jsonl",
            "--misses",
            "5",
            "--homophone-snapshot",
            "snap.tsv",
            "--check",
        ]);
        assert_eq!(args.replay, Some(PathBuf::from("log.jsonl")));
        assert_eq!(args.misses, 5);
        assert_eq!(args.homophone_snapshot, Some(PathBuf::from("snap.tsv")));
        assert!(args.check);
        assert!(args.inputs.is_empty());
    }
}
