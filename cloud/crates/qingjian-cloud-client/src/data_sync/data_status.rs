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
    /// 只增不减：本 `DataSync` 实例内不会自动恢复，用户在服务器上重新打开后须重建 `DataSync`
    /// （Mac 切开关时重启服务、iOS 键盘发现 cloud.toml 变化重开会话）。
    /// `Feature::Sync` 同时覆盖学习数据与配置两项。所有项被停掉后 `last_ok_ms` 仍会刷新，壳要按 `disabled` 判断。
    pub disabled: Vec<Feature>,

    /// 令牌被拒（401）：停在这里等重新登录。
    pub unauthorized: bool,
}
