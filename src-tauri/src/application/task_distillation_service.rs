//! Evidence selection and semantic validation for Run and Task Distillation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const RULES_VERSION: &str = "task-distillation-rules-v2";
pub const RESULT_SCHEMA_VERSION: &str = "task-distillation-result-v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub id: String,
    pub revision: String,
    pub locator: String,
    pub content_hash: String,
    pub role: String,
    #[serde(default)]
    pub evidence_state: String,
    pub omission: Option<SourceOmission>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceOmission {
    pub omitted_chars: usize,
    pub strategy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillationOwner {
    pub task_id: String,
    pub task_revision: i64,
    pub run_id: Option<String>,
    pub run_revision: Option<i64>,
    pub work_log_entry_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillationInput {
    pub projection_kind: String,
    pub owner: DistillationOwner,
    pub primary_source: Option<ExactSource>,
    pub missing_primary_reason: Option<String>,
    #[serde(default)]
    pub sources: Vec<ExactSource>,
    #[serde(default)]
    pub affected_semantic_ids: Vec<String>,
    pub expected_projection_revision: i64,
    #[serde(default)]
    pub expected_journey_revision: i64,
    #[serde(default)]
    pub semantic_target_revisions: BTreeMap<String, i64>,
    pub source_set_hash: String,
    pub rules_version: String,
    pub locale: String,
    pub repair_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCitation {
    #[serde(rename = "type")]
    pub source_type: String,
    pub id: String,
    pub revision: String,
    pub locator: String,
    #[serde(default)]
    pub quote: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistilledClaim {
    pub id: String,
    pub kind: String,
    pub statement: String,
    pub actor: String,
    pub epistemic_state: String,
    pub status: String,
    pub topic_key: Option<String>,
    #[serde(default)]
    pub sources: Vec<SourceCitation>,
    #[serde(default)]
    pub contradicted_by: Vec<SourceCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreparedDetail {
    pub before: Option<String>,
    pub after: Option<String>,
    pub reason: Option<String>,
    #[serde(default)]
    pub evidence_claim_ids: Vec<String>,
    pub result: Option<String>,
    pub current_status: String,
    #[serde(default)]
    pub links: Vec<SourceCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeChange {
    pub action: String,
    pub candidate_id: String,
    #[serde(default)]
    pub target_ids: Vec<String>,
    #[serde(default)]
    pub expected_target_revisions: BTreeMap<String, i64>,
    #[serde(default)]
    pub claim_ids: Vec<String>,
    pub detail: PreparedDetail,
    #[serde(default)]
    pub sources: Vec<SourceCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipChange {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub reason: Option<String>,
    #[serde(default)]
    pub sources: Vec<SourceCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicState {
    pub topic_key: String,
    pub status: String,
    pub current_node_id: Option<String>,
    pub replacement_node_id: Option<String>,
    #[serde(default)]
    pub sources: Vec<SourceCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionSnapshot {
    pub id: String,
    pub completion_id: String,
    pub completion_revision: String,
    pub status: String,
    #[serde(default)]
    pub claim_ids: Vec<String>,
    #[serde(default)]
    pub unresolved: Vec<String>,
    pub verification_state: String,
    #[serde(default)]
    pub sources: Vec<SourceCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkLogSection {
    pub kind: String,
    #[serde(default)]
    pub claim_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkLogView {
    #[serde(default)]
    pub sections: Vec<WorkLogSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDependency {
    pub source: SourceCitation,
    #[serde(default)]
    pub claim_ids: Vec<String>,
    #[serde(default)]
    pub relationship_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DistillationResult {
    #[serde(default)]
    pub claims: Vec<DistilledClaim>,
    #[serde(default)]
    pub node_changes: Vec<NodeChange>,
    #[serde(default)]
    pub relationship_changes: Vec<RelationshipChange>,
    #[serde(default)]
    pub topic_states: Vec<TopicState>,
    #[serde(default)]
    pub completion_snapshots: Vec<CompletionSnapshot>,
    pub work_log_view: WorkLogView,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<SourceDependency>,
}

pub fn content_hash(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

pub fn bounded_manifest_excerpt(value: &str, limit: usize) -> (String, Option<SourceOmission>) {
    let total = value.chars().count();
    if total <= limit {
        return (value.to_owned(), None);
    }
    let head = limit.saturating_mul(3) / 4;
    let tail = limit.saturating_sub(head);
    let omitted = total.saturating_sub(head + tail);
    let excerpt = format!(
        "{}\n[… {omitted} source characters omitted from the middle …]\n{}",
        value.chars().take(head).collect::<String>(),
        value.chars().skip(total - tail).collect::<String>()
    );
    (
        excerpt,
        Some(SourceOmission {
            omitted_chars: omitted,
            strategy: "head_and_tail".into(),
        }),
    )
}

pub fn source_set_hash(primary: Option<&ExactSource>, sources: &[ExactSource]) -> String {
    let mut identity = primary
        .into_iter()
        .chain(sources)
        .map(|source| {
            format!(
                "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{:?}",
                source.source_type,
                source.id,
                source.revision,
                source.locator,
                source.content_hash,
                source.role,
                source.evidence_state,
                source.subject
            )
        })
        .collect::<Vec<_>>();
    identity.sort();
    content_hash(&identity.join("\n"))
}

pub fn stable_semantic_id(task_id: &str, kind: &str, topic: &str, occurrence: &str) -> String {
    let normalized = |value: &str| {
        value
            .trim()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let digest = Sha256::digest(
        format!(
            "{}\0{}\0{}\0{}",
            task_id,
            normalized(kind),
            normalized(topic),
            normalized(occurrence)
        )
        .as_bytes(),
    );
    format!("semantic:{:.24x}", digest)
}

pub fn build_run_input(
    owner: DistillationOwner,
    final_report: Option<&str>,
    missing_reason: Option<&str>,
    completed_items: impl IntoIterator<Item = ExactSource>,
    expected_projection_revision: i64,
    locale: &str,
) -> Result<DistillationInput, String> {
    let primary_source = final_report
        .filter(|value| !value.trim().is_empty())
        .map(|report| ExactSource {
            source_type: "run_final_report".into(),
            id: owner.run_id.clone().unwrap_or_default(),
            revision: owner.run_revision.unwrap_or_default().to_string(),
            locator: "final_report".into(),
            content_hash: content_hash(report),
            role: "primary".into(),
            evidence_state: "reported".into(),
            omission: None,
            subject: None,
            excerpt: report.to_owned(),
        });
    if primary_source.is_none() && missing_reason.is_none() {
        return Err("missingPrimaryReason is required when no final report exists".into());
    }
    let sources = completed_items
        .into_iter()
        .filter(|source| {
            matches!(
                source.role.as_str(),
                "corroborates"
                    | "missing_material_fact"
                    | "contradicts"
                    | "task_context"
                    | "deletion_tombstone"
            ) && !matches!(
                source.source_type.as_str(),
                "stream_delta" | "retry_control" | "continue_control" | "approval_control"
            )
        })
        .collect::<Vec<_>>();
    let source_set_hash = source_set_hash(primary_source.as_ref(), &sources);
    Ok(DistillationInput {
        projection_kind: "work_log".into(),
        owner,
        primary_source,
        missing_primary_reason: missing_reason.map(str::to_owned),
        sources,
        affected_semantic_ids: Vec::new(),
        expected_projection_revision,
        expected_journey_revision: 0,
        semantic_target_revisions: BTreeMap::new(),
        source_set_hash,
        rules_version: RULES_VERSION.into(),
        locale: locale.into(),
        repair_reason: None,
    })
}

fn citation_key(source_type: &str, id: &str, revision: &str, locator: &str) -> String {
    format!("{source_type}\0{id}\0{revision}\0{locator}")
}

fn citation_source<'a>(
    manifest: &'a HashMap<String, &ExactSource>,
    citation: &SourceCitation,
) -> Option<&'a ExactSource> {
    manifest
        .get(&citation_key(
            &citation.source_type,
            &citation.id,
            &citation.revision,
            &citation.locator,
        ))
        .copied()
}

fn retained_source_segments(source: &ExactSource) -> Vec<&str> {
    if source.omission.is_none() {
        return vec![&source.excerpt];
    }
    if let Some((head, rest)) = source.excerpt.split_once("\n[… ") {
        if let Some((_, tail)) = rest.split_once(" …]\n") {
            return vec![head, tail];
        }
    }
    vec![&source.excerpt]
}

fn exact_quote_is_bound(citation: &SourceCitation, source: &ExactSource) -> bool {
    !citation.quote.trim().is_empty()
        && !citation.quote.contains("source characters omitted")
        && source.excerpt.contains(citation.quote.trim())
}

pub fn validate_result(
    input: &DistillationInput,
    result: &DistillationResult,
) -> Result<(), String> {
    if input.rules_version != RULES_VERSION {
        return Err("repair_required: rules version is incompatible".into());
    }
    if source_set_hash(input.primary_source.as_ref(), &input.sources) != input.source_set_hash {
        return Err("stale_source: source-set hash does not match the manifest".into());
    }
    let manifest = input
        .primary_source
        .iter()
        .chain(&input.sources)
        .map(|source| {
            (
                citation_key(
                    &source.source_type,
                    &source.id,
                    &source.revision,
                    &source.locator,
                ),
                source,
            )
        })
        .collect::<HashMap<_, _>>();
    let claims = result
        .claims
        .iter()
        .map(|claim| (claim.id.as_str(), claim))
        .collect::<HashMap<_, _>>();
    if claims.len() != result.claims.len() {
        return Err("duplicate claim id".into());
    }
    let check_citations = |citations: &[SourceCitation], label: &str| -> Result<(), String> {
        if citations.is_empty() {
            return Err(format!("{label} requires an exact source"));
        }
        for citation in citations {
            let Some(source) = citation_source(&manifest, citation) else {
                return Err(format!(
                    "{label} cites a source absent from the input manifest"
                ));
            };
            if !exact_quote_is_bound(citation, source) {
                return Err(format!(
                    "{label} requires an exact quote present in its source"
                ));
            }
        }
        Ok(())
    };
    for claim in &result.claims {
        if claim.id.trim().is_empty() || claim.statement.trim().is_empty() {
            return Err("claim id and statement are required".into());
        }
        check_citations(&claim.sources, &format!("claim {}", claim.id))?;
        if !matches!(
            claim.epistemic_state.as_str(),
            "suggested" | "decided" | "attempted" | "performed" | "observed" | "verified"
        ) || !matches!(claim.actor.as_str(), "user" | "ai" | "tool")
            || !matches!(
                claim.status.as_str(),
                "current" | "contradicted" | "historical" | "unknown"
            )
        {
            return Err("unknown claim state".into());
        }
        if claim.epistemic_state == "decided" && claim.actor != "user" {
            return Err("claim cannot be decided without a user decision".into());
        }

        if claim.epistemic_state == "verified" {
            let observed = claim.sources.iter().any(|citation| {
                citation_source(&manifest, citation).is_some_and(|source| {
                    source.source_type == "run_completed_item"
                        && source.evidence_state == "successful_check"
                        && citation.quote.trim() == claim.statement.trim()
                        && source.excerpt.trim() == citation.quote.trim()
                })
            });
            if !observed {
                return Err(format!(
                    "claim {} cannot be verified without completed evidence",
                    claim.id
                ));
            }
        }
        if claim.epistemic_state == "decided" {
            let explicit = claim.sources.iter().any(|citation| {
                citation_source(&manifest, citation).is_some_and(|source| {
                    matches!(
                        source.source_type.as_str(),
                        "task_decision" | "user_decision"
                    ) && source.evidence_state == "explicit_decision"
                        && citation.quote.trim() == claim.statement.trim()
                        && source.excerpt.trim() == citation.quote.trim()
                })
            });
            if !explicit {
                return Err(format!(
                    "claim {} cannot be decided from prose or AI suggestion",
                    claim.id
                ));
            }
        }
        check_citations(
            &claim.contradicted_by,
            &format!("claim {} contradiction", claim.id),
        )
        .or_else(|error| {
            if claim.contradicted_by.is_empty() {
                Ok(())
            } else {
                Err(error)
            }
        })?;
        for citation in &claim.contradicted_by {
            if let Some(source) = citation_source(&manifest, citation) {
                if source.evidence_state != "failed_check"
                    || source
                        .subject
                        .as_ref()
                        .is_some_and(|subject| claim.topic_key.as_ref() != Some(subject))
                {
                    return Err("contradiction requires the same canonical check subject".into());
                }
            }
        }
        if !claim.contradicted_by.is_empty() && claim.status != "contradicted" {
            return Err(format!(
                "claim {} must expose material contradictory evidence",
                claim.id
            ));
        }
    }
    let mut candidate_ids = BTreeSet::new();
    for change in &result.node_changes {
        if !matches!(
            change.action.as_str(),
            "add" | "enrich" | "merge" | "omit" | "supersede"
        ) {
            return Err("unknown semantic action".into());
        }
        if change.action != "omit" {
            check_citations(
                &change.sources,
                &format!("node change {}", change.candidate_id),
            )?;
            if !change.detail.links.is_empty() {
                check_citations(
                    &change.detail.links,
                    &format!("node change {} detail", change.candidate_id),
                )?;
            }
            if change
                .claim_ids
                .iter()
                .any(|id| !claims.contains_key(id.as_str()))
            {
                return Err("node change references an unknown claim".into());
            }
        }
        if matches!(change.action.as_str(), "merge" | "enrich" | "supersede")
            && change.target_ids.is_empty()
        {
            return Err(format!("{} requires a target", change.action));
        }
        let expected = change
            .expected_target_revisions
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let targets = change.target_ids.iter().cloned().collect::<BTreeSet<_>>();
        if targets.len() != change.target_ids.len()
            || (change.action == "enrich" && targets.len() != 1)
        {
            return Err("invalid semantic targets".into());
        }
        if change.action == "add" && !targets.is_empty() {
            return Err("add cannot mutate targets".into());
        }
        for (target, revision) in &change.expected_target_revisions {
            if input.semantic_target_revisions.get(target) != Some(revision) {
                return Err("target revision absent from generation context".into());
            }
        }

        if expected != targets {
            return Err(format!(
                "{} requires an exact expected revision for every target",
                change.action
            ));
        }
        if (!input.affected_semantic_ids.is_empty()
            || input
                .repair_reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("targeted source change:")))
            && change.action != "add"
            && !targets
                .iter()
                .all(|id| input.affected_semantic_ids.contains(id))
        {
            return Err("node change targets semantic nodes outside the affected subgraph".into());
        }
        let node_claims = change
            .claim_ids
            .iter()
            .filter_map(|id| claims.get(id.as_str()))
            .collect::<Vec<_>>();
        if change.detail.current_status == "verified"
            && !node_claims
                .iter()
                .any(|claim| claim.epistemic_state == "verified")
        {
            return Err("verified node status requires a verified claim".into());
        }
        if matches!(change.detail.current_status.as_str(), "adopted" | "decided")
            && !node_claims
                .iter()
                .any(|claim| claim.epistemic_state == "decided")
        {
            return Err("adopted node status requires an explicit decided claim".into());
        }
        if change.action == "supersede" && change.detail.reason.as_deref().is_none_or(str::is_empty)
        {
            return Err("supersede requires an explicit supported reason".into());
        }
        if change.action == "supersede"
            && !change
                .claim_ids
                .iter()
                .filter_map(|id| claims.get(id.as_str()))
                .any(|claim| {
                    claim.epistemic_state == "decided"
                        && claim.sources.iter().any(|citation| {
                            change.sources.contains(citation)
                                && change
                                    .detail
                                    .reason
                                    .as_ref()
                                    .is_some_and(|reason| citation.quote.contains(reason))
                        })
                })
        {
            return Err("supersede requires a matching explicit decision and quoted reason".into());
        }
        if change.action != "omit" && !candidate_ids.insert(change.candidate_id.as_str()) {
            return Err("duplicate semantic candidate id".into());
        }
    }
    for relationship in &result.relationship_changes {
        check_citations(
            &relationship.sources,
            &format!("relationship {}", relationship.id),
        )?;
        if !matches!(
            relationship.kind.as_str(),
            "supports" | "verifies" | "contradicts" | "supersedes" | "merged_into"
        ) {
            return Err("unknown relationship kind".into());
        }
        if relationship.kind == "supersedes"
            && !result.node_changes.iter().any(|change| {
                change.action == "supersede"
                    && change.candidate_id == relationship.from
                    && change.target_ids.contains(&relationship.to)
                    && change.detail.reason == relationship.reason
            })
        {
            return Err("supersession relationship must match its validated target change".into());
        }
        if matches!(relationship.kind.as_str(), "verifies" | "supersedes") {
            let expected = if relationship.kind == "verifies" {
                "verified"
            } else {
                "decided"
            };
            if !result
                .node_changes
                .iter()
                .filter(|node| {
                    node.candidate_id == relationship.from
                        || node.target_ids.contains(&relationship.from)
                })
                .flat_map(|node| &node.claim_ids)
                .filter_map(|id| claims.get(id.as_str()))
                .any(|claim| {
                    claim.epistemic_state == expected
                        && claim
                            .sources
                            .iter()
                            .any(|citation| relationship.sources.contains(citation))
                })
            {
                return Err(
                    "authoritative relationship requires its matching validated claim".into(),
                );
            }
        }
        if relationship.from == relationship.to {
            return Err("semantic relationship cannot self-reference".into());
        }
        if (!input.affected_semantic_ids.is_empty()
            || input
                .repair_reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("targeted source change:")))
            && [&relationship.from, &relationship.to].iter().any(|id| {
                !candidate_ids.contains(id.as_str()) && !input.affected_semantic_ids.contains(id)
            })
        {
            return Err(
                "relationship change reaches outside the affected semantic subgraph".into(),
            );
        }
        if matches!(relationship.kind.as_str(), "supersedes" | "verifies")
            && relationship.reason.as_deref().is_none_or(str::is_empty)
        {
            return Err(format!(
                "{} relationship requires explicit evidence and reason",
                relationship.kind
            ));
        }
    }
    for topic in &result.topic_states {
        check_citations(&topic.sources, &format!("topic {}", topic.topic_key))?;
        if topic.status == "adopted" && topic.current_node_id.is_none() {
            return Err("adopted topic requires a current node".into());
        }
        if matches!(
            topic.status.as_str(),
            "adopted" | "superseded" | "withdrawn"
        ) {
            let explicit = topic.sources.iter().any(|citation| {
                citation_source(&manifest, citation).is_some_and(|source| {
                    source.evidence_state == "explicit_decision"
                        && matches!(
                            source.source_type.as_str(),
                            "task_decision" | "user_decision"
                        )
                })
            });
            let decided_node = topic.current_node_id.as_ref().is_some_and(|node| {
                result
                    .node_changes
                    .iter()
                    .filter(|change| {
                        &change.candidate_id == node || change.target_ids.contains(node)
                    })
                    .flat_map(|change| &change.claim_ids)
                    .filter_map(|id| claims.get(id.as_str()))
                    .any(|claim| {
                        claim.epistemic_state == "decided"
                            && claim.topic_key.as_deref() == Some(topic.topic_key.as_str())
                            && claim
                                .sources
                                .iter()
                                .any(|citation| topic.sources.contains(citation))
                    })
            });
            if !explicit || !decided_node {
                return Err(
                    "adopted topic requires an exact explicit decision and matching decided claim"
                        .into(),
                );
            }
        }
        if topic.status == "superseded"
            && !result.node_changes.iter().any(|change| {
                change.action == "supersede"
                    && topic.replacement_node_id.as_ref() == Some(&change.candidate_id)
                    && topic
                        .current_node_id
                        .as_ref()
                        .is_some_and(|id| change.target_ids.contains(id))
            })
        {
            return Err("superseded topic requires its explicit replacement action".into());
        }
        if topic.status == "withdrawn"
            && !topic
                .sources
                .iter()
                .filter_map(|citation| citation_source(&manifest, citation))
                .any(|source| {
                    let Some((kind, raw)) = source.excerpt.split_once(": ") else {
                        return false;
                    };
                    let payload =
                        serde_json::from_str::<serde_json::Value>(raw).unwrap_or_default();
                    let action = payload
                        .get("action")
                        .or_else(|| payload.get("status"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(kind);
                    let subject = payload
                        .get("topicKey")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(kind);
                    matches!(action, "withdraw" | "withdrawn") && subject == topic.topic_key
                })
        {
            return Err("withdrawn topic requires canonical withdrawal for that topic".into());
        }
        if !matches!(
            topic.status.as_str(),
            "adopted" | "superseded" | "withdrawn" | "unresolved"
        ) {
            return Err("unknown topic status".into());
        }
    }
    for snapshot in &result.completion_snapshots {
        check_citations(
            &snapshot.sources,
            &format!("completion snapshot {}", snapshot.id),
        )?;
        let exact = snapshot.sources.iter().any(|citation| {
            citation_source(&manifest, citation).is_some_and(|source| {
                source.source_type == "task_completion"
                    && source.evidence_state == "explicit_completion"
                    && source.id == snapshot.completion_id
                    && source.revision == snapshot.completion_revision
            })
        });
        if !exact {
            return Err(
                "completion snapshot requires its exact canonical completion source".into(),
            );
        }
        if !matches!(
            snapshot.status.as_str(),
            "current" | "historical_after_reopen" | "superseded_by_later_completion"
        ) || !matches!(
            snapshot.verification_state.as_str(),
            "recorded" | "partially_verified" | "verified"
        ) {
            return Err("invalid completion state".into());
        }
        if snapshot.claim_ids.is_empty()
            || snapshot.claim_ids.iter().any(|id| {
                claims.get(id.as_str()).is_none_or(|claim| {
                    !claim.sources.iter().any(|citation| {
                        citation.source_type == "task_completion"
                            && citation.id == snapshot.completion_id
                            && citation.revision == snapshot.completion_revision
                            && citation.quote == claim.statement
                    })
                })
            })
        {
            return Err(
                "completion snapshot requires claims bound to its canonical completion".into(),
            );
        }
        let verified = snapshot
            .claim_ids
            .iter()
            .filter_map(|id| claims.get(id.as_str()))
            .filter(|claim| claim.epistemic_state == "verified")
            .count();
        if (snapshot.verification_state == "verified"
            && (verified != snapshot.claim_ids.len() || !snapshot.unresolved.is_empty()))
            || (snapshot.verification_state == "partially_verified" && verified == 0)
        {
            return Err("completion record does not establish verification".into());
        }
    }
    // A failed attempt is always retained even when the model omits it from
    // its report claims. A later success on the same exact command changes its
    // role to failed_approach, never evidence against unrelated work.
    for source in manifest.values().filter(|source| {
        matches!(
            source.evidence_state.as_str(),
            "failed_check" | "failed_approach"
        )
    }) {
        let represented = retained_source_segments(source).iter().all(|segment| {
            result.claims.iter().any(|claim| {
                let own_failure =
                    matches!(claim.epistemic_state.as_str(), "observed" | "attempted")
                        && claim.sources.iter().any(|citation| {
                            citation.id == source.id
                                && citation.quote == claim.statement
                                && citation.quote == *segment
                                && exact_quote_is_bound(citation, source)
                        });
                let contrary = claim.status == "contradicted"
                    && claim.contradicted_by.iter().any(|citation| {
                        citation.id == source.id
                            && citation.quote == *segment
                            && exact_quote_is_bound(citation, source)
                    });
                own_failure || contrary
            })
        });
        if !represented {
            return Err("known contradictory or failed evidence was omitted".into());
        }
    }
    Ok(())
}

pub fn canonicalize_semantic_ids(
    input: &DistillationInput,
    result: &mut DistillationResult,
) -> Result<(), String> {
    let mut claim_ids = HashMap::new();
    let mut seen_claims = BTreeSet::new();
    for claim in &mut result.claims {
        let source = claim
            .sources
            .first()
            .ok_or("claim requires source anchor")?;
        let canonical = stable_semantic_id(
            &input.owner.task_id,
            &format!("claim:{}", claim.kind),
            claim.topic_key.as_deref().unwrap_or(""),
            &format!(
                "{}:{}:{}:{}",
                source.source_type, source.id, source.locator, source.quote
            ),
        );
        if !seen_claims.insert(canonical.clone()) {
            return Err("canonical claim anchor collision".into());
        }
        claim_ids.insert(claim.id.clone(), canonical.clone());
        claim.id = canonical;
    }
    let rewrite = |ids: &mut Vec<String>| {
        for id in ids {
            if let Some(canonical) = claim_ids.get(id) {
                *id = canonical.clone();
            }
        }
    };
    for change in &mut result.node_changes {
        rewrite(&mut change.claim_ids);
        rewrite(&mut change.detail.evidence_claim_ids);
    }
    for section in &mut result.work_log_view.sections {
        rewrite(&mut section.claim_ids);
    }
    for snapshot in &mut result.completion_snapshots {
        snapshot.id = stable_semantic_id(
            &input.owner.task_id,
            "completion_snapshot",
            &snapshot.completion_id,
            &snapshot.completion_revision,
        );
        rewrite(&mut snapshot.claim_ids);
    }
    for dependency in &mut result.dependencies {
        rewrite(&mut dependency.claim_ids);
    }
    let claims = result
        .claims
        .iter()
        .map(|claim| (claim.id.clone(), claim.clone()))
        .collect::<HashMap<_, _>>();
    let mut replacements = HashMap::new();
    for change in &mut result.node_changes {
        if matches!(change.action.as_str(), "enrich" | "merge") {
            let target = change
                .target_ids
                .first()
                .ok_or("enrichment requires target")?
                .clone();
            replacements.insert(change.candidate_id.clone(), target.clone());
            change.candidate_id = target;
        }
        if !matches!(change.action.as_str(), "add" | "supersede") {
            continue;
        }
        let Some(claim) = change.claim_ids.iter().find_map(|id| claims.get(id)) else {
            continue;
        };
        let occurrence = claim
            .sources
            .first()
            .map(|source| {
                format!(
                    "{}:{}:{}:{}",
                    source.source_type,
                    source.id,
                    source.locator,
                    source.quote.trim()
                )
            })
            .ok_or("new semantic node requires a source anchor")?;
        let canonical = stable_semantic_id(
            &input.owner.task_id,
            &claim.kind,
            claim.topic_key.as_deref().unwrap_or(&claim.kind),
            &occurrence,
        );
        if replacements.values().any(|existing| existing == &canonical) {
            return Err(
                "semantic anchor collision: distinct candidates resolve to one stable node".into(),
            );
        }
        replacements.insert(change.candidate_id.clone(), canonical.clone());
        change.candidate_id = canonical;
    }
    for relationship in &mut result.relationship_changes {
        if let Some(value) = replacements.get(&relationship.from) {
            relationship.from = value.clone();
        }
        if let Some(value) = replacements.get(&relationship.to) {
            relationship.to = value.clone();
        }
    }
    for topic in &mut result.topic_states {
        if let Some(value) = topic
            .current_node_id
            .as_ref()
            .and_then(|id| replacements.get(id))
        {
            topic.current_node_id = Some(value.clone());
        }
        if let Some(value) = topic
            .replacement_node_id
            .as_ref()
            .and_then(|id| replacements.get(id))
        {
            topic.replacement_node_id = Some(value.clone());
        }
    }
    Ok(())
}

pub fn factual_fallback(input: &DistillationInput, run_status: &str) -> DistillationResult {
    let mut result = DistillationResult::default();
    result.warnings.push(if input.primary_source.is_some() {
        "Semantic Distillation is unavailable; showing saved Run facts.".into()
    } else {
        format!(
            "No final report is available: {}",
            input
                .missing_primary_reason
                .as_deref()
                .unwrap_or("unknown reason")
        )
    });
    if let Some(primary) = &input.primary_source {
        let claim_id =
            stable_semantic_id(&input.owner.task_id, "result", "run outcome", &primary.id);
        result.claims.push(DistilledClaim {
            id: claim_id.clone(),
            kind: "result".into(),
            statement: primary.excerpt.clone(),
            actor: "ai".into(),
            epistemic_state: "performed".into(),
            status: "unknown".into(),
            topic_key: None,
            sources: vec![SourceCitation {
                source_type: primary.source_type.clone(),
                id: primary.id.clone(),
                revision: primary.revision.clone(),
                locator: primary.locator.clone(),
                quote: primary.excerpt.clone(),
            }],
            contradicted_by: Vec::new(),
        });
        result.work_log_view.sections.push(WorkLogSection {
            kind: "outcome".into(),
            claim_ids: vec![claim_id],
        });
    }
    for source in input.sources.iter().filter(|source| {
        matches!(
            source.evidence_state.as_str(),
            "failed_check" | "failed_approach"
        )
    }) {
        for (index, segment) in retained_source_segments(source).into_iter().enumerate() {
            let claim_id = stable_semantic_id(
                &input.owner.task_id,
                "attempt",
                "saved failure",
                &format!("{}:{index}", source.id),
            );
            result.claims.push(DistilledClaim {
                id: claim_id.clone(),
                kind: "attempt".into(),
                statement: segment.to_owned(),
                actor: "tool".into(),
                epistemic_state: "observed".into(),
                status: if source.evidence_state == "failed_approach" {
                    "historical"
                } else {
                    "unknown"
                }
                .into(),
                topic_key: source.subject.clone(),
                sources: vec![SourceCitation {
                    source_type: source.source_type.clone(),
                    id: source.id.clone(),
                    revision: source.revision.clone(),
                    locator: source.locator.clone(),
                    quote: segment.to_owned(),
                }],
                contradicted_by: vec![],
            });
            result.work_log_view.sections.push(WorkLogSection {
                kind: "unresolved".into(),
                claim_ids: vec![claim_id],
            });
        }
        if let Some(omission) = &source.omission {
            result.warnings.push(format!(
                "{}: {} source characters omitted from context; full source retained.",
                source.id, omission.omitted_chars
            ));
        }
    }
    result
        .warnings
        .push(format!("Saved Run status: {run_status}"));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> DistillationOwner {
        DistillationOwner {
            task_id: "task".into(),
            task_revision: 3,
            run_id: Some("run".into()),
            run_revision: Some(4),
            work_log_entry_id: Some("log".into()),
        }
    }
    fn evidence(role: &str) -> ExactSource {
        ExactSource {
            source_type: "run_completed_item".into(),
            id: "check".into(),
            revision: "1".into(),
            locator: "content".into(),
            content_hash: content_hash("passed"),
            role: role.into(),
            evidence_state: "successful_check".into(),
            omission: None,
            subject: None,
            excerpt: "passed".into(),
        }
    }
    fn citation(source: &ExactSource) -> SourceCitation {
        SourceCitation {
            source_type: source.source_type.clone(),
            id: source.id.clone(),
            revision: source.revision.clone(),
            locator: source.locator.clone(),
            quote: source.excerpt.clone(),
        }
    }

    #[test]
    fn report_is_primary_and_routine_controls_are_omitted() {
        let mut control = evidence("corroborates");
        control.source_type = "retry_control".into();
        let input = build_run_input(
            owner(),
            Some("Implemented export"),
            None,
            [evidence("corroborates"), control, evidence("routine")],
            0,
            "en",
        )
        .unwrap();
        assert_eq!(
            input.primary_source.unwrap().source_type,
            "run_final_report"
        );
        assert_eq!(input.sources.len(), 1);
    }

    #[test]
    fn primary_report_keeps_late_final_decisions_and_conditions() {
        let report = format!(
            "{}FINAL DECISION: SQLite only when offline mode is enabled",
            "context ".repeat(80)
        );
        let input = build_run_input(owner(), Some(&report), None, [], 0, "en").unwrap();
        assert_eq!(input.primary_source.unwrap().excerpt, report);
    }

    #[test]
    fn missing_report_requires_a_reason_and_fallback_stays_factual() {
        assert!(build_run_input(owner(), None, None, [], 0, "en").is_err());
        let input = build_run_input(
            owner(),
            None,
            Some("run failed"),
            [evidence("corroborates")],
            0,
            "en",
        )
        .unwrap();
        let fallback = factual_fallback(&input, "failed");
        assert!(fallback.claims.is_empty());
        assert!(fallback
            .warnings
            .iter()
            .any(|warning| warning.contains("No final report")));
    }

    #[test]
    fn validation_rejects_unsupported_decision_and_verification() {
        let input = build_run_input(
            owner(),
            Some("Choose SQLite"),
            None,
            [evidence("corroborates")],
            0,
            "en",
        )
        .unwrap();
        let primary = input.primary_source.as_ref().unwrap();
        let citation = citation(primary);
        let mut result = DistillationResult::default();
        result.claims.push(DistilledClaim {
            id: "c".into(),
            kind: "decision".into(),
            statement: "SQLite".into(),
            actor: "ai".into(),
            epistemic_state: "decided".into(),
            status: "current".into(),
            topic_key: Some("storage".into()),
            sources: vec![citation.clone()],
            contradicted_by: vec![],
        });
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("cannot be decided"));
        result.claims[0].epistemic_state = "verified".into();
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("completed evidence"));
    }

    #[test]
    fn stable_ids_ignore_display_text_but_preserve_occurrence() {
        assert_eq!(
            stable_semantic_id("t", "Decision", "Storage", "first"),
            stable_semantic_id("t", " decision ", "storage", "first")
        );
        assert_ne!(
            stable_semantic_id("t", "decision", "storage", "first"),
            stable_semantic_id("t", "decision", "storage", "second")
        );
    }

    #[test]
    fn unrelated_completed_source_cannot_upgrade_a_claim_to_verified() {
        let mut unrelated = evidence("corroborates");
        unrelated.excerpt = "database migration passed".into();
        let input = build_run_input(
            owner(),
            Some("Rendered keyboard focus"),
            None,
            [unrelated],
            0,
            "en",
        )
        .unwrap();
        let evidence = &input.sources[0];
        let result = DistillationResult {
            claims: vec![DistilledClaim {
                id: "c".into(),
                kind: "evidence".into(),
                statement: "Keyboard focus verified".into(),
                actor: "tool".into(),
                epistemic_state: "verified".into(),
                status: "current".into(),
                topic_key: None,
                sources: vec![citation(evidence)],
                contradicted_by: vec![],
            }],
            ..Default::default()
        };
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("completed evidence"));
    }

    #[test]
    fn opposite_polarity_words_do_not_grant_verification() {
        let mut failed = evidence("corroborates");
        failed.excerpt = "export check failed".into();
        failed.evidence_state = "failed_check".into();
        let input = build_run_input(
            owner(),
            Some("Export check passed"),
            None,
            [failed],
            0,
            "en",
        )
        .unwrap();
        let source = &input.sources[0];
        let result = DistillationResult {
            claims: vec![DistilledClaim {
                id: "c".into(),
                kind: "evidence".into(),
                statement: "export check failed".into(),
                actor: "tool".into(),
                epistemic_state: "verified".into(),
                status: "current".into(),
                topic_key: None,
                sources: vec![citation(source)],
                contradicted_by: vec![],
            }],
            ..Default::default()
        };
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("cannot be verified"));
    }

    #[test]
    fn report_and_check_contradiction_must_remain_visible() {
        let mut failed = evidence("contradicts");
        failed.excerpt = "export check failed".into();
        failed.evidence_state = "failed_check".into();
        let input = build_run_input(
            owner(),
            Some("Export check passed"),
            None,
            [failed],
            0,
            "en",
        )
        .unwrap();
        let primary = input.primary_source.as_ref().unwrap();
        let failed = &input.sources[0];
        let report = citation(primary);
        let contradiction = citation(failed);
        let mut result = DistillationResult {
            claims: vec![DistilledClaim {
                id: "c".into(),
                kind: "result".into(),
                statement: "Export check passed".into(),
                actor: "ai".into(),
                epistemic_state: "performed".into(),
                status: "current".into(),
                topic_key: None,
                sources: vec![report],
                contradicted_by: vec![],
            }],
            ..Default::default()
        };
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("contradict"));
        result.claims[0].status = "contradicted".into();
        result.claims[0].contradicted_by = vec![contradiction];
        validate_result(&input, &result).unwrap();
    }

    #[test]
    fn routine_control_only_candidates_produce_no_semantic_node() {
        let mut retry = evidence("corroborates");
        retry.source_type = "retry_control".into();
        let input = build_run_input(owner(), None, Some("interrupted"), [retry], 0, "en").unwrap();
        let result = factual_fallback(&input, "interrupted");
        assert!(result.node_changes.is_empty());
        assert!(result.claims.is_empty());
    }

    #[test]
    fn target_mutations_require_complete_cas_and_affected_scope() {
        let input =
            build_run_input(owner(), Some("Implemented export"), None, [], 0, "en").unwrap();
        let source = input.primary_source.as_ref().unwrap();
        let citation = citation(source);
        let claim = DistilledClaim {
            id: "claim".into(),
            kind: "result".into(),
            statement: "Implemented export".into(),
            actor: "ai".into(),
            epistemic_state: "performed".into(),
            status: "current".into(),
            topic_key: Some("export".into()),
            sources: vec![citation.clone()],
            contradicted_by: vec![],
        };
        let mut result = DistillationResult {
            claims: vec![claim],
            node_changes: vec![NodeChange {
                action: "enrich".into(),
                candidate_id: "candidate".into(),
                target_ids: vec!["existing".into()],
                expected_target_revisions: BTreeMap::new(),
                claim_ids: vec!["claim".into()],
                detail: PreparedDetail::default(),
                sources: vec![citation],
            }],
            ..Default::default()
        };
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("expected revision"));
        result.node_changes[0]
            .expected_target_revisions
            .insert("existing".into(), 1);
        let mut scoped = input.clone();
        scoped
            .semantic_target_revisions
            .insert("existing".into(), 1);
        scoped.affected_semantic_ids = vec!["other".into()];
        assert!(validate_result(&scoped, &result)
            .unwrap_err()
            .contains("outside the affected"));
    }

    #[test]
    fn adoption_and_completion_require_canonical_evidence() {
        let mut input =
            build_run_input(owner(), Some("Use SQLite and finish"), None, [], 0, "en").unwrap();
        let decision_text = "choice: {\"body\":\"Use SQLite\"}";
        let completion_text = "Evidence: cargo test passed\nReport: finished";
        input.sources.push(ExactSource {
            source_type: "task_decision".into(),
            id: "decision".into(),
            revision: "3".into(),
            locator: "payload_json".into(),
            content_hash: content_hash(decision_text),
            role: "task_context".into(),
            evidence_state: "explicit_decision".into(),
            omission: None,
            subject: None,
            excerpt: decision_text.into(),
        });
        input.sources.push(ExactSource {
            source_type: "task_completion".into(),
            id: "completion".into(),
            revision: "3".into(),
            locator: "evidence".into(),
            content_hash: content_hash(completion_text),
            role: "task_context".into(),
            evidence_state: "explicit_completion".into(),
            omission: None,
            subject: None,
            excerpt: completion_text.into(),
        });
        input.source_set_hash = source_set_hash(input.primary_source.as_ref(), &input.sources);
        let decision = citation(&input.sources[0]);
        let completion = citation(&input.sources[1]);
        let claim = DistilledClaim {
            id: "decision-claim".into(),
            kind: "decision".into(),
            statement: decision_text.into(),
            actor: "user".into(),
            epistemic_state: "decided".into(),
            status: "current".into(),
            topic_key: Some("storage".into()),
            sources: vec![decision.clone()],
            contradicted_by: vec![],
        };
        let change = NodeChange {
            action: "add".into(),
            candidate_id: "decision-node".into(),
            target_ids: vec![],
            expected_target_revisions: BTreeMap::new(),
            claim_ids: vec![claim.id.clone()],
            detail: PreparedDetail::default(),
            sources: vec![decision.clone()],
        };
        let mut result = DistillationResult {
            claims: vec![claim],
            node_changes: vec![change],
            topic_states: vec![TopicState {
                topic_key: "storage".into(),
                status: "adopted".into(),
                current_node_id: Some("decision-node".into()),
                replacement_node_id: None,
                sources: vec![citation(input.primary_source.as_ref().unwrap())],
            }],
            completion_snapshots: vec![CompletionSnapshot {
                id: "snapshot".into(),
                completion_id: "completion".into(),
                completion_revision: "3".into(),
                status: "current".into(),
                claim_ids: vec!["decision-claim".into()],
                unresolved: vec![],
                verification_state: "recorded".into(),
                sources: vec![],
            }],
            ..Default::default()
        };
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("explicit decision"));
        result.topic_states[0].sources = vec![decision];
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("exact source"));
        result.completion_snapshots[0].sources = vec![completion.clone()];
        result.claims.push(DistilledClaim {
            id: "completion-claim".into(),
            kind: "result".into(),
            statement: completion.quote.clone(),
            actor: "user".into(),
            epistemic_state: "observed".into(),
            status: "current".into(),
            topic_key: None,
            sources: vec![completion],
            contradicted_by: vec![],
        });
        result.completion_snapshots[0].claim_ids = vec!["completion-claim".into()];
        validate_result(&input, &result).unwrap();
    }

    #[test]
    fn authoritative_node_labels_and_supersession_cannot_bypass_claim_validation() {
        let input = build_run_input(owner(), Some("Suggested SQLite"), None, [], 0, "en").unwrap();
        let source = citation(input.primary_source.as_ref().unwrap());
        let mut result = DistillationResult {
            claims: vec![DistilledClaim {
                id: "claim".into(),
                kind: "idea".into(),
                statement: source.quote.clone(),
                actor: "ai".into(),
                epistemic_state: "suggested".into(),
                status: "unknown".into(),
                topic_key: Some("storage".into()),
                sources: vec![source.clone()],
                contradicted_by: vec![],
            }],
            node_changes: vec![NodeChange {
                action: "add".into(),
                candidate_id: "node".into(),
                target_ids: vec![],
                expected_target_revisions: BTreeMap::new(),
                claim_ids: vec!["claim".into()],
                detail: PreparedDetail {
                    current_status: "verified".into(),
                    ..Default::default()
                },
                sources: vec![source.clone()],
            }],
            ..Default::default()
        };
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("verified node status"));
        result.node_changes[0].detail.current_status = "adopted".into();
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("adopted node status"));
        result.node_changes[0].detail.current_status = "unresolved".into();
        result.relationship_changes.push(RelationshipChange {
            id: "edge".into(),
            kind: "supersedes".into(),
            from: "node".into(),
            to: "other".into(),
            reason: Some("Suggested SQLite".into()),
            sources: vec![source],
        });
        assert!(validate_result(&input, &result)
            .unwrap_err()
            .contains("validated target change"));
    }

    #[test]
    fn canonical_node_identity_ignores_model_labels_and_rewrites_links() {
        let input =
            build_run_input(owner(), Some("Implemented export"), None, [], 0, "en").unwrap();
        let source = input.primary_source.as_ref().unwrap();
        let citation = citation(source);
        let mut result = DistillationResult {
            claims: vec![DistilledClaim {
                id: "claim".into(),
                kind: "result".into(),
                statement: "Implemented export".into(),
                actor: "ai".into(),
                epistemic_state: "performed".into(),
                status: "current".into(),
                topic_key: Some("export".into()),
                sources: vec![citation.clone()],
                contradicted_by: vec![],
            }],
            node_changes: vec![NodeChange {
                action: "add".into(),
                candidate_id: "model-random".into(),
                target_ids: vec![],
                expected_target_revisions: BTreeMap::new(),
                claim_ids: vec!["claim".into()],
                detail: PreparedDetail {
                    current_status: "unresolved".into(),
                    ..Default::default()
                },
                sources: vec![citation.clone()],
            }],
            relationship_changes: vec![RelationshipChange {
                id: "link".into(),
                kind: "supports".into(),
                from: "model-random".into(),
                to: "existing".into(),
                reason: None,
                sources: vec![citation],
            }],
            ..Default::default()
        };
        canonicalize_semantic_ids(&input, &mut result).unwrap();
        assert_ne!(result.node_changes[0].candidate_id, "model-random");
        assert_eq!(
            result.relationship_changes[0].from,
            result.node_changes[0].candidate_id
        );
    }

    #[test]
    fn canonical_node_identity_ignores_source_revision_and_rejects_anchor_collisions() {
        let input =
            build_run_input(owner(), Some("Implemented export"), None, [], 0, "en").unwrap();
        let source = input.primary_source.as_ref().unwrap();
        let first = citation(source);
        let mut revised = first.clone();
        revised.revision = "99".into();
        let first_id = stable_semantic_id(
            "task",
            "result",
            "export",
            &format!(
                "{}:{}:{}:{}",
                first.source_type, first.id, first.locator, first.quote
            ),
        );
        let revised_id = stable_semantic_id(
            "task",
            "result",
            "export",
            &format!(
                "{}:{}:{}:{}",
                revised.source_type, revised.id, revised.locator, revised.quote
            ),
        );
        assert_eq!(first_id, revised_id);
        let claim = DistilledClaim {
            id: "one".into(),
            kind: "result".into(),
            statement: "Implemented export".into(),
            actor: "ai".into(),
            epistemic_state: "performed".into(),
            status: "current".into(),
            topic_key: Some("export".into()),
            sources: vec![first.clone()],
            contradicted_by: vec![],
        };
        let mut result = DistillationResult {
            claims: vec![
                claim.clone(),
                DistilledClaim {
                    id: "two".into(),
                    ..claim
                },
            ],
            node_changes: vec![
                NodeChange {
                    action: "add".into(),
                    candidate_id: "one".into(),
                    target_ids: vec![],
                    expected_target_revisions: BTreeMap::new(),
                    claim_ids: vec!["one".into()],
                    detail: PreparedDetail::default(),
                    sources: vec![first.clone()],
                },
                NodeChange {
                    action: "add".into(),
                    candidate_id: "two".into(),
                    target_ids: vec![],
                    expected_target_revisions: BTreeMap::new(),
                    claim_ids: vec!["two".into()],
                    detail: PreparedDetail::default(),
                    sources: vec![first],
                },
            ],
            ..Default::default()
        };
        assert!(canonicalize_semantic_ids(&input, &mut result)
            .unwrap_err()
            .contains("collision"));
    }
}
