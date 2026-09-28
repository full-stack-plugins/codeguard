use codeguard_runtime::verify_approval_signature;

fn decode<const N: usize>(value: &str) -> [u8; N] {
    assert_eq!(value.len(), N * 2);
    let bytes: Vec<u8> = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    bytes.try_into().unwrap()
}

#[test]
fn rfc8032_empty_message_vector_and_mutations_are_verified() {
    // 来源：RFC 8032 §7.1 TEST 1；仅公钥与签名，不引入测试私钥。
    let key = decode("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    let signature = decode(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );
    assert!(verify_approval_signature(&key, b"", &signature));
    assert!(!verify_approval_signature(&key, b"changed", &signature));
    let mut changed = signature;
    changed[0] ^= 1;
    assert!(!verify_approval_signature(&key, b"", &changed));
    assert!(!verify_approval_signature(&[0; 32], b"", &signature));
}

#[test]
fn invalid_signatures_and_oversized_messages_cannot_pass() {
    let key = decode("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    assert!(!verify_approval_signature(&key, b"", &[0; 64]));
    assert!(!verify_approval_signature(
        &key,
        &vec![0; 9 * 1024 + 1],
        &[0; 64]
    ));
}
