//! npm原生组件的脱敏投影；保留advisory编号与锁版本，不授予漏洞确认。
use crate::npm_audit_probe_result::NpmAuditProbeResult;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
pub(crate) fn project(result: &NpmAuditProbeResult) -> Vec<Value> {
    result.parsed.as_ref().map_or_else(Vec::new,|parsed|parsed.components.iter().map(|component|{
        let nodes:Vec<Value>=result.locked_nodes.iter().filter(|node|component.node_locations.contains(&node.location)).map(|node|json!({"node_ref":format!("{:x}",Sha256::digest(node.location.as_bytes())),"resolved_version":node.resolved_version})).collect();
        json!({"component_ref":format!("{:x}",Sha256::digest(component.native_name.as_bytes())),"severity":component.severity,"advisory_sources":component.advisory_sources,"nodes":nodes})
    }).collect())
}
