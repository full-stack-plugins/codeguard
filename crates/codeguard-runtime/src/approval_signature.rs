//! 受控的密码学验签原语；不解析策略、不加载信任库、不提供签发能力。

use ring::signature::{ED25519, UnparsedPublicKey};

/// 核验给定公钥下的 Ed25519 消息签名；输入消息超过 9 KiB 时拒绝。
///
/// 公钥来源由受保护宿主负责；返回真仅表示密码学签名有效，不代表批准或门禁通过。
#[must_use]
pub fn verify_approval_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> bool {
    message.len() <= 9 * 1024
        && UnparsedPublicKey::new(&ED25519, public_key)
            .verify(message, signature)
            .is_ok()
}
