//! 同步一轮要用的两个学习数据接口；产品里是 [`Client`]，测试换成内存里的假服务器。

use qingjian_cloud_proto::{LearningPage, LearningPush};

use crate::{Client, ClientError};

pub(crate) trait LearningRemote: Send {
    /// 推一批增量，返回服务器最新的 `seq`。
    fn push_learning(&self, push: &LearningPush) -> Result<u64, ClientError>;

    /// 拉 `since` 之后变过的行。
    fn learning(&self, since: u64, limit: usize) -> Result<LearningPage, ClientError>;
}

impl LearningRemote for Client {
    fn push_learning(&self, push: &LearningPush) -> Result<u64, ClientError> {
        Client::push_learning(self, push)
    }

    fn learning(&self, since: u64, limit: usize) -> Result<LearningPage, ClientError> {
        Client::learning(self, since, limit)
    }
}
