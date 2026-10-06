//! 账号操作的「代数」与加入阶段：建空间、输码申请与轮询都在别的线程里完成，结果晚到时可能已经退出、取消或换了账号。
//! 每次开始加入、取消、退出、拿到会话都让代数前进；请求带着发出时的代数，晚到的旧结果代数对不上就丢掉。

/// 加入进行到哪一步。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Idle,

    /// 已输码申请，正在等旧设备（手机）上允许。
    Joining,
}

#[derive(Debug, Default)]
pub struct AccountFlow {
    generation: u64,

    phase: Phase,
}

impl AccountFlow {
    /// 当前代数：后台请求发出时记下。
    pub fn current(&self) -> u64 {
        self.generation
    }

    /// 事件是不是当前代数的；晚到的旧结果返回 `false`。
    pub fn accepts(&self, generation: u64) -> bool {
        generation == self.generation
    }

    /// 正在等旧设备允许（或建空间正在进行）。
    pub fn signing_in(&self) -> bool {
        self.phase != Phase::Idle
    }

    /// 开始加入：已经在加入就不允许，返回这次的代数。
    pub fn start_join(&mut self) -> Option<u64> {
        if self.signing_in() {
            return None;
        }
        self.phase = Phase::Joining;
        self.generation += 1;
        Some(self.generation)
    }

    /// 加入结束（成功、失败或用户取消），不动代数。
    pub fn finish(&mut self) {
        self.phase = Phase::Idle;
    }

    /// 取消加入：丢掉正在等的轮询结果。
    pub fn cancel(&mut self) {
        self.phase = Phase::Idle;
        self.generation += 1;
    }

    /// 退出、令牌失效、登录完成：旧令牌上发出的请求结果都作废。
    pub fn invalidate(&mut self) {
        self.generation += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_join_at_a_time() {
        let mut flow = AccountFlow::default();
        let first = flow.start_join().unwrap();
        assert!(flow.signing_in());
        assert_eq!(flow.start_join(), None);
        assert!(flow.accepts(first));
        flow.finish();
        assert!(!flow.signing_in());
        assert!(flow.start_join().is_some());
    }

    #[test]
    fn join_result_arriving_after_cancel_is_dropped() {
        let mut flow = AccountFlow::default();
        let gen_ = flow.start_join().unwrap();
        flow.cancel();
        assert!(!flow.accepts(gen_));
        assert!(!flow.signing_in());
    }

    #[test]
    fn stale_sign_out_cannot_log_out_a_new_session() {
        let mut flow = AccountFlow::default();
        // 旧令牌上发出的请求带当时的代数
        let old_request = flow.current();
        // 加入成功：代数前进
        let gen_ = flow.start_join().unwrap();
        assert!(flow.accepts(gen_));
        flow.finish();
        flow.invalidate();
        assert!(!flow.accepts(old_request));
        assert!(!flow.accepts(gen_));
        assert!(flow.accepts(flow.current()));
    }

    #[test]
    fn old_canceled_join_cannot_land_on_the_new_one() {
        let mut flow = AccountFlow::default();
        let old_join = flow.start_join().unwrap();
        flow.cancel();
        let new_join = flow.start_join().unwrap();
        // 旧轮询的结果这时才到
        assert!(!flow.accepts(old_join));
        assert!(flow.accepts(new_join));
        assert!(flow.signing_in());
    }
}
