//! OWASP 包标识与本轮 Maven 原生依赖图的精确坐标候选归属。

use std::collections::BTreeSet;

use crate::{MavenDependencyTree, OwaspDependencyCheckReport};

/// 单条 OWASP 观察与 Maven 图的归属结果；仅精确坐标候选，不代表制品或数据库认证。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwaspAttributionState {
    /// 唯一 Maven group/artifact/version/type/classifier 坐标与图中非根节点一致。
    ExactCoordinateCandidate,
    /// 报告没有 Maven PURL。
    MissingMavenPurl,
    /// Maven PURL 包含本适配器无法无损解释的编码或限定符。
    UnsupportedPurl,
    /// 同一漏洞观察对应多个不同 Maven PURL。
    ConflictingPurls,
    /// 原生图无该精确坐标。
    NotInGraph,
    /// 原生图中出现多个相同坐标，不能唯一归属。
    AmbiguousGraphCoordinate,
}

/// 以报告中原始观察顺序定位归属，不根据相似文件名或消息推断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwaspAttribution {
    /// 原报告 `advisories` 中的索引。
    pub observation_index: usize,
    /// 候选归属状态。
    pub state: OwaspAttributionState,
    /// 唯一精确坐标候选在 Maven 图中的节点索引，其它状态为空。
    pub graph_node_index: Option<usize>,
}

/// OWASP 与本轮 Maven 制品字节摘要的比较；即使一致，也只是待认证的身份候选。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwaspArtifactBindingState {
    /// 坐标尚未唯一归属或归属列表不一致。
    AttributionUnavailable,
    /// OWASP 依赖项没有文件 SHA-256。
    OwaspDigestMissing,
    /// Maven 图对应节点没有可用的文件 SHA-256。
    MavenDigestMissing,
    /// 任一摘要不是 SHA-256 十六进制值。
    DigestEvidenceInvalid,
    /// 两边的文件字节摘要不同。
    DigestMismatch,
    /// 两边摘要相同；仍需认证报告、仓库和数据库来源。
    ExactDigestCandidate,
}

/// 先从本次报告和图重新计算精确归属，再按节点顺序比较两边的制品摘要。
/// `maven_digests` 的来源由调用方核验；此函数不把相同摘要升级为 CVE 判定。
pub fn bind_owasp_advisories_to_artifact_digests(
    report: &OwaspDependencyCheckReport,
    graph: &MavenDependencyTree,
    maven_digests: &[Option<String>],
) -> Vec<OwaspArtifactBindingState> {
    let attributions = attribute_owasp_advisories_to_maven_graph(report, graph);
    report
        .advisories
        .iter()
        .enumerate()
        .map(|(index, advisory)| {
            let Some(attribution) = attributions.get(index).filter(|attribution| {
                attribution.observation_index == index
                    && attribution.state == OwaspAttributionState::ExactCoordinateCandidate
            }) else {
                return OwaspArtifactBindingState::AttributionUnavailable;
            };
            let Some(report_digest) = advisory.dependency_sha256.as_deref() else {
                return OwaspArtifactBindingState::OwaspDigestMissing;
            };
            let Some(maven_digest) = attribution
                .graph_node_index
                .and_then(|node| maven_digests.get(node))
                .and_then(Option::as_deref)
            else {
                return OwaspArtifactBindingState::MavenDigestMissing;
            };
            if !sha256(report_digest) || !sha256(maven_digest) {
                return OwaspArtifactBindingState::DigestEvidenceInvalid;
            }
            if report_digest.eq_ignore_ascii_case(maven_digest) {
                OwaspArtifactBindingState::ExactDigestCandidate
            } else {
                OwaspArtifactBindingState::DigestMismatch
            }
        })
        .collect()
}

fn sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// 将 OWASP 原生漏洞逐条与同轮 Maven 依赖图做精确坐标候选归属。
/// 数据库、文件摘要、扫描范围与可信来源仍须在调用方独立验证，成功也不能签发漏洞结论。
pub fn attribute_owasp_advisories_to_maven_graph(
    report: &OwaspDependencyCheckReport,
    graph: &MavenDependencyTree,
) -> Vec<OwaspAttribution> {
    report
        .advisories
        .iter()
        .enumerate()
        .map(|(observation_index, advisory)| {
            let mut coordinates = BTreeSet::new();
            for id in &advisory.package_ids {
                if !id.starts_with("pkg:maven/") {
                    continue;
                }
                let Some(coordinate) = parse_exact_maven_purl(id) else {
                    return OwaspAttribution {
                        observation_index,
                        state: OwaspAttributionState::UnsupportedPurl,
                        graph_node_index: None,
                    };
                };
                coordinates.insert(coordinate);
            }
            let Some(coordinate) = coordinates.iter().next() else {
                return OwaspAttribution {
                    observation_index,
                    state: OwaspAttributionState::MissingMavenPurl,
                    graph_node_index: None,
                };
            };
            if coordinates.len() != 1 {
                return OwaspAttribution {
                    observation_index,
                    state: OwaspAttributionState::ConflictingPurls,
                    graph_node_index: None,
                };
            }
            let matching: Vec<_> = graph
                .nodes
                .iter()
                .enumerate()
                .skip(1)
                .filter(|(_, node)| {
                    coordinate.group_id == node.group_id
                        && coordinate.artifact_id == node.artifact_id
                        && coordinate.version == node.version
                        && coordinate.artifact_type == node.artifact_type
                        && coordinate.classifier == node.classifier
                })
                .map(|(index, _)| index)
                .collect();
            let (state, graph_node_index) = match matching.as_slice() {
                [] => (OwaspAttributionState::NotInGraph, None),
                [index] => (
                    OwaspAttributionState::ExactCoordinateCandidate,
                    Some(*index),
                ),
                _ => (OwaspAttributionState::AmbiguousGraphCoordinate, None),
            };
            OwaspAttribution {
                observation_index,
                state,
                graph_node_index,
            }
        })
        .collect()
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct MavenCoordinate<'a> {
    group_id: &'a str,
    artifact_id: &'a str,
    version: &'a str,
    artifact_type: &'a str,
    classifier: &'a str,
}

fn parse_exact_maven_purl(value: &str) -> Option<MavenCoordinate<'_>> {
    let body = value.strip_prefix("pkg:maven/")?;
    if body.contains(['%', '#']) {
        return None;
    }
    let (path, qualifiers) = body.split_once('?').unwrap_or((body, ""));
    let (group_id, artifact_version) = path.split_once('/')?;
    let (artifact_id, version) = artifact_version.split_once('@')?;
    if !coordinate(group_id) || !coordinate(artifact_id) || !coordinate(version) {
        return None;
    }
    let mut artifact_type = "jar";
    let mut classifier = "";
    let mut seen_type = false;
    let mut seen_classifier = false;
    if !qualifiers.is_empty() {
        for item in qualifiers.split('&') {
            let (key, value) = item.split_once('=')?;
            if !coordinate(value) {
                return None;
            }
            match key {
                "type" if !seen_type => {
                    artifact_type = value;
                    seen_type = true;
                }
                "classifier" if !seen_classifier => {
                    classifier = value;
                    seen_classifier = true;
                }
                _ => return None,
            }
        }
    } else if body.contains('?') {
        return None;
    }
    Some(MavenCoordinate {
        group_id,
        artifact_id,
        version,
        artifact_type,
        classifier,
    })
}

fn coordinate(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
