//! 「同一输入串下选过」的加分。原来是排序键里压在上下文得分之前的一级，选过一次「邮箱」上文怎么写都是它第一；
//! 改成对数加分，强上文翻得过只选过一两次的词，选过很多次的仍压得住弱上文。
//! β=8 是 `--replay` 词首选扫出来的（一次选择值 5.5 nat，强上文超过它才翻；见 cloud/docs/plans/2026-10-04-context-prediction.md Task 3）。

/// 加分系数 β：加分 = β · ln(1 + 次数)。
pub const CHOICE_BONUS: f64 = 8.0;

/// 选过 `count` 次换成的得分加成。
pub fn choice_bonus(count: u32, weight: f64) -> f64 {
    weight * (1.0 + f64::from(count)).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_with_the_log_of_the_count() {
        assert_eq!(choice_bonus(0, CHOICE_BONUS), 0.0);
        assert!(choice_bonus(1, CHOICE_BONUS) < choice_bonus(5, CHOICE_BONUS));
        assert!((choice_bonus(1, 1.0) - std::f64::consts::LN_2).abs() < 1e-12);
    }
}
