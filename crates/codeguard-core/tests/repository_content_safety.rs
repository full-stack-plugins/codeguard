use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use codeguard_core::has_unencrypted_openssh_ed25519_private_key;

fn ssh_string(bytes: &[u8], output: &mut Vec<u8>) {
    output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn fixture_with_padding(add_padding: bool) -> Vec<u8> {
    let public_bytes = [0xa5_u8; 32];
    let mut public = Vec::new();
    ssh_string(b"ssh-ed25519", &mut public);
    ssh_string(&public_bytes, &mut public);
    let mut private = Vec::new();
    private.extend_from_slice(&0x1234_5678_u32.to_be_bytes());
    private.extend_from_slice(&0x1234_5678_u32.to_be_bytes());
    ssh_string(b"ssh-ed25519", &mut private);
    ssh_string(&public_bytes, &mut private);
    let mut key_material = [0x5a_u8; 64];
    key_material[32..].copy_from_slice(&public_bytes);
    ssh_string(&key_material, &mut private);
    let comment_length = (8 - (private.len() + 4) % 8) % 8;
    ssh_string(&vec![b'x'; comment_length], &mut private);
    if add_padding {
        private.extend((1..=8).map(|value| value as u8));
    }
    let mut envelope = b"openssh-key-v1\0".to_vec();
    ssh_string(b"none", &mut envelope);
    ssh_string(b"none", &mut envelope);
    ssh_string(b"", &mut envelope);
    envelope.extend_from_slice(&1_u32.to_be_bytes());
    ssh_string(&public, &mut envelope);
    ssh_string(&private, &mut envelope);
    format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----\n",
        STANDARD.encode(envelope)
    )
    .into_bytes()
}

fn fixture() -> Vec<u8> {
    fixture_with_padding(true)
}

#[test]
fn recognizes_structurally_complete_openssh_private_key_without_needing_a_sensitive_name() {
    assert!(has_unencrypted_openssh_ed25519_private_key(&fixture()));
    assert!(has_unencrypted_openssh_ed25519_private_key(
        &fixture_with_padding(false)
    ));
    assert!(has_unencrypted_openssh_ed25519_private_key(
        &[b"# documentation\n".as_slice(), &fixture()].concat()
    ));
    let crlf = String::from_utf8(fixture()).unwrap().replace('\n', "\r\n");
    assert!(has_unencrypted_openssh_ed25519_private_key(crlf.as_bytes()));
}

#[test]
fn rejects_header_only_invalid_base64_and_truncated_envelopes() {
    assert!(!has_unencrypted_openssh_ed25519_private_key(
        b"-----BEGIN OPENSSH PRIVATE KEY-----\nexample\n-----END OPENSSH PRIVATE KEY-----\n"
    ));
    assert!(!has_unencrypted_openssh_ed25519_private_key(
        b"-----BEGIN OPENSSH PRIVATE KEY-----\n!!!!\n-----END OPENSSH PRIVATE KEY-----\n"
    ));
    let mut truncated = fixture();
    truncated.truncate(truncated.len() - 39);
    assert!(!has_unencrypted_openssh_ed25519_private_key(&truncated));

    let text = String::from_utf8(fixture()).unwrap();
    let body = text.lines().nth(1).unwrap();
    let mut decoded = STANDARD.decode(body).unwrap();
    *decoded.last_mut().unwrap() = 0;
    let damaged = format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----\n",
        STANDARD.encode(decoded)
    );
    assert!(!has_unencrypted_openssh_ed25519_private_key(
        damaged.as_bytes()
    ));
}
