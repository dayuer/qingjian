//! `<对象 id>/materials.jsonl` 的一行：「记一笔」存下的一条待整理素材。原话原样（名字、时间都留着），最多
//! `MAX_MEMORY_TEXT_BYTES` 字节；上传前替换对象名字与脱敏都不改这里存的原文。

use qingjian_cloud_proto::MemoryKind;
use serde::{Deserialize, Serialize};

use super::MaterialSource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Material {
    /// 本机生成的 32 位十六进制随机 id，上传时作 `MemoryItem.client_id`，2C 下发的卡靠它回指素材。
    pub client_id: String,

    /// 恒为 `note`（2B 的 `MemoryKind::Note`）。
    pub kind: MemoryKind,

    pub text: String,

    /// 用户点「记」的时间，Unix 秒。
    pub at: i64,

    pub source: MaterialSource,

    /// 已经交给素笺云（2B 上传成功）。
    #[serde(default)]
    pub uploaded: bool,

    /// 已经整理成卡（2C 下发并确认）；之后再留 30 天给用户对照。
    #[serde(default)]
    pub processed: bool,

    /// 标成已整理的时间，Unix 秒；30 天的保留期从这里算。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processed_at: Option<i64>,
}
