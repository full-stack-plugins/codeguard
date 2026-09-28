use serde::Deserialize;
/// 发行签名封装，原始载荷文本参与域分隔签名。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedDistributionEnvelope {
    pub schema_version: String,
    pub key_id: String,
    pub release_json: String,
    pub signature_hex: String,
}
