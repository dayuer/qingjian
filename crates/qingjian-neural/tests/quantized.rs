//! 8 位权重（`.qjm` 的 `Q8GF` 节）对真模型的验收：与 fp16 旧格式、与第一步的假量化 int8 参照逐条比首选。
//!
//! 模型不进仓库，用环境变量给三件套目录，没给就跳过：
//! `QJ_TONGBIAN`（含章·通变 fp16），`QJ_TONGBIAN_INT8REF`（假量化 int8 + 嵌入，可选）。
//! `QJ_TONGBIAN=… QJ_TONGBIAN_INT8REF=… cargo test --release -p qingjian-neural --test quantized`

use std::path::{Path, PathBuf};

use qingjian_format::Metadata;
use qingjian_neural::{CharScorer, P2c, qjm};

/// 20 条拼音：对话、书面、地名、口语助词都有。
const KEYS: [&str; 20] = [
    "wojintianxiangqushanghai",
    "zhegecanguandedianhuashishenme",
    "nimenjidianxiaban",
    "qingbangwoyudingyijianshuangrenfang",
    "zhegewentiwomenmingtianzaitaolun",
    "tianqiyubaoshuomingtianhuixiayu",
    "zhonghuarenmingongheguo",
    "beijingdaxuedetushuguan",
    "womenyiqiquchifanba",
    "nikanguozhebudianyingma",
    "jiqixuexidemoxingxunlian",
    "zhegeruanjiandeyonghutiyanbucuo",
    "wangluolianjieyouwenti",
    "haodewozhidaole",
    "mingtianzaoshangbadianjian",
    "zhefangzidezujinyoudiangui",
    "shurufadecikuxuyaogengxin",
    "tadeyijianwomenyinggaizunzhong",
    "jingjixingjiudianyoumeiyoukongfang",
    "xiexienidebangzhu",
];

fn env_dir(name: &str) -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os(name)?);
    dir.join(qjm::WEIGHTS_FILE).is_file().then_some(dir)
}

fn top_choices(scorer: &CharScorer) -> Vec<String> {
    let p2c = P2c::new(scorer.model(), scorer.vocab()).expect("P2C 模型");
    KEYS.iter()
        .map(|keys| {
            p2c.convert(keys, 4, 32)
                .unwrap()
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_default()
        })
        .collect()
}

fn agreement(a: &[String], b: &[String]) -> f64 {
    let same = a.iter().zip(b).filter(|(x, y)| x == y).count();
    same as f64 / a.len() as f64
}

fn pack(dir: &Path, name: &str, quantize: bool) -> PathBuf {
    let out_dir = std::env::temp_dir().join("qingjian-neural-tests/quantized");
    std::fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.join(name);
    qjm::pack(dir, &out, &Metadata::default(), quantize).unwrap();
    out
}

#[test]
fn eight_bit_matches_fp16_and_the_fake_quantized_reference() {
    let Some(dir) = env_dir("QJ_TONGBIAN") else {
        eprintln!("没给 QJ_TONGBIAN，跳过");
        return;
    };
    let fp16_file = pack(&dir, "fp16.qjm", false);
    let q8_file = pack(&dir, "q8.qjm", true);
    // 8 位那一节比 safetensors 原文小一半左右
    let (fp16_size, q8_size) = (
        std::fs::metadata(&fp16_file).unwrap().len(),
        std::fs::metadata(&q8_file).unwrap().len(),
    );
    assert!(q8_size * 10 < fp16_size * 6, "{q8_size} vs {fp16_size}");

    // fp16 旧格式（SAFT 节）与三件套目录照常加载
    let fp16 = top_choices(&CharScorer::load(&fp16_file).unwrap());
    let by_dir = top_choices(&CharScorer::load(&dir).unwrap());
    assert_eq!(fp16, by_dir);

    let q8 = top_choices(&CharScorer::load(&q8_file).unwrap());
    let vs_fp16 = agreement(&q8, &fp16);
    eprintln!("8 位 对 fp16 首选一致 {vs_fp16:.2}");
    assert!(vs_fp16 >= 0.95, "{q8:?}\n{fp16:?}");

    if let Some(reference) = env_dir("QJ_TONGBIAN_INT8REF") {
        let fake = top_choices(&CharScorer::load(&reference).unwrap());
        let vs_fake = agreement(&q8, &fake);
        eprintln!("8 位 对 假量化 int8 首选一致 {vs_fake:.2}");
        assert!(vs_fake >= 0.95, "{q8:?}\n{fake:?}");
    }
}

/// 按块算目标字 log 概率（`score_p2c` 走的路）与摊开整张 log-softmax 再 gather 同值。
#[test]
fn chunked_target_log_probs_match_the_full_softmax() {
    let Some(dir) = env_dir("QJ_TONGBIAN") else {
        eprintln!("没给 QJ_TONGBIAN，跳过");
        return;
    };
    let scorer = CharScorer::load(&pack(&dir, "q8-chunk.qjm", true)).unwrap();
    let model = scorer.model();
    let vocab = scorer.vocab();
    let head: Vec<u32> = std::iter::once(0)
        .chain(vocab.encode("zhegecanguan"))
        .collect();
    let cache = model.prefix_cache(&head).unwrap();
    // 19 行：跨过两个块边界
    let ids: Vec<u32> = vocab.encode("这个餐馆的电话是什么这个参观的电话是什么呀");
    let (b, t) = (1, 19);
    let idx = candle_core::Tensor::from_vec(ids[..b * t].to_vec(), (b, t), model.device()).unwrap();
    let targets: Vec<u32> = ids[1..=b * t].to_vec();
    let chunked = model
        .target_log_probs_after(&cache, &idx, &targets)
        .unwrap();
    let full = model
        .log_probs_after(&cache, &idx)
        .unwrap()
        .to_vec3::<f32>()
        .unwrap();
    for (i, (&got, &target)) in chunked.iter().zip(&targets).enumerate() {
        let want = full[0][i][target as usize];
        assert!((got - want).abs() < 1e-4, "{i}: {got} vs {want}");
    }
}
