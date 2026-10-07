//! 回放里最近几次上屏各几个字：日志里的撤销（retract）要按真实使用的样子先把那次上屏及其后的字逐字退格删掉。

/// 最多记几次；引擎自己的最近上屏表比这短，多记无妨。
const CAPACITY: usize = 32;

/// (日志里的上屏 `id`, 上屏的字数)，按上屏顺序。
#[derive(Debug, Default)]
pub struct RecentCommits(Vec<(u64, usize)>);

impl RecentCommits {
    pub fn push(&mut self, id: u64, chars: usize) {
        if self.0.len() >= CAPACITY {
            self.0.remove(0);
        }
        self.0.push((id, chars));
    }

    /// 撤销 `of` 那次：返回从它（最近一条同 `id` 的）到最后一次上屏共几个字，并把它们从表里拿掉；回放里没上屏过它就是 `None`。
    pub fn erase_from(&mut self, of: u64) -> Option<usize> {
        let index = self.0.iter().rposition(|&(id, _)| id == of)?;
        let chars = self.0[index..].iter().map(|&(_, chars)| chars).sum();
        self.0.truncate(index);
        Some(chars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erasing_counts_the_retracted_commit_and_everything_after_it() {
        let mut recent = RecentCommits::default();
        recent.push(1, 2);
        recent.push(2, 2);
        recent.push(3, 1);
        // 撤销 id 2：它与之后的 id 3 都被删掉，共 3 个字
        assert_eq!(recent.erase_from(2), Some(3));
        // 删掉的不再参与下一次撤销
        assert_eq!(recent.erase_from(3), None);
        assert_eq!(recent.erase_from(1), Some(2));
    }

    #[test]
    fn ids_restart_each_session_so_the_latest_one_wins() {
        let mut recent = RecentCommits::default();
        recent.push(1, 4);
        recent.push(1, 2);
        assert_eq!(recent.erase_from(1), Some(2));
    }
}
