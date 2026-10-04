//! 账号操作的「代数」与登录阶段：登录窗口的回调、换令牌与开关请求都在别的线程里完成，结果晚到时可能已经退出、取消或换了账号。
//! 每次登录开始、取消、退出、登录完成都让代数前进；请求带着发出时的代数，晚到的旧结果代数对不上就丢掉。

/// 登录进行到哪一步。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Idle,

    /// 网页登录窗口开着。
    WebOpen,

    /// 窗口已回跳，正在用一次性码换令牌。
    Exchanging,
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

    /// 登录窗口开着或正在换令牌。
    pub fn signing_in(&self) -> bool {
        self.phase != Phase::Idle
    }

    /// 开始新的登录：已经在登录（含换令牌）就不允许，返回这次登录的代数。
    pub fn start_login(&mut self) -> Option<u64> {
        if self.signing_in() {
            return None;
        }
        self.phase = Phase::WebOpen;
        self.generation += 1;
        Some(self.generation)
    }

    /// 窗口回跳了，进入换令牌。窗口不是开着的（已取消）返回 `false`。
    pub fn callback_received(&mut self) -> bool {
        if self.phase != Phase::WebOpen {
            return false;
        }
        self.phase = Phase::Exchanging;
        true
    }

    /// 登录结束（成功、失败或窗口没打开），不动代数。
    pub fn finish(&mut self) {
        self.phase = Phase::Idle;
    }

    /// 取消登录：丢掉正在进行的窗口与换令牌的结果。
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
    fn only_one_login_at_a_time() {
        let mut flow = AccountFlow::default();
        let first = flow.start_login().unwrap();
        assert!(flow.signing_in());
        assert_eq!(flow.start_login(), None);
        flow.callback_received();
        assert_eq!(flow.start_login(), None, "换令牌过程中不能再开登录");
        assert!(flow.accepts(first));
        flow.finish();
        assert!(!flow.signing_in());
        assert!(flow.start_login().is_some());
    }

    #[test]
    fn sign_in_arriving_after_cancel_is_dropped() {
        let mut flow = AccountFlow::default();
        let gen_ = flow.start_login().unwrap();
        flow.callback_received();
        flow.cancel();
        assert!(!flow.accepts(gen_));
        assert!(!flow.signing_in());
    }

    #[test]
    fn stale_sign_out_cannot_log_out_a_new_session() {
        let mut flow = AccountFlow::default();
        // 旧令牌上发出的请求带当时的代数
        let old_request = flow.current();
        // 重新登录成功：代数前进
        let gen_ = flow.start_login().unwrap();
        flow.callback_received();
        assert!(flow.accepts(gen_));
        flow.finish();
        flow.invalidate();
        assert!(!flow.accepts(old_request));
        assert!(!flow.accepts(gen_));
        assert!(flow.accepts(flow.current()));
    }

    #[test]
    fn old_canceled_callback_does_not_close_the_new_login() {
        let mut flow = AccountFlow::default();
        let old_login = flow.start_login().unwrap();
        flow.cancel();
        let new_login = flow.start_login().unwrap();
        // 旧窗口的 CanceledLogin 回调这时才到
        assert!(!flow.accepts(old_login));
        assert!(flow.accepts(new_login));
        assert!(flow.signing_in());
    }
}
