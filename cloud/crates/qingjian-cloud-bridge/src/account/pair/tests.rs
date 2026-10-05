//! 匹配码这条路的说法：码不对、空间满各有各的中文，其余沿用通用的那句。

use qingjian_cloud_client::ClientError;

use super::pair_message;

#[test]
fn a_bad_code_and_a_full_space_say_what_to_do_next() {
    assert_eq!(
        pair_message(&ClientError::BadCode(String::new())),
        "匹配码不对或已经过期，请重新输一张"
    );
    assert_eq!(
        pair_message(&ClientError::DeviceLimit(String::new())),
        "空间里的设备已经满了，先在旧设备上删一台再加"
    );
}

#[test]
fn everything_else_keeps_the_generic_wording() {
    assert_eq!(
        pair_message(&ClientError::Unreachable(String::new())),
        "连不上服务器，检查网络后再试"
    );
}
