//! 学习数据同步的状态，壳显示在菜单里。

use qingjian_cloud_proto::Feature;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataStatus {
    /// 上次成功的时间，Unix 毫秒。
    pub last_ok_ms: Option<i64>,

    /// 收件箱在等输入法合并（输入法没在用，或者装的不是打了补丁的版本）。
    pub waiting_for_ime: bool,

    /// 上次失败的原因；成功后清掉。
    pub error: Option<String>,

    /// 出现过配置冲突（较旧的一份已存成备份）。
    pub config_conflict: bool,

    /// 服务器上没开、已经停掉的功能（403）；壳据此把对应开关显示为关。
    pub disabled: Vec<Feature>,

    /// 令牌被拒（401）：停在这里等重新登录。
    pub unauthorized: bool,
}
