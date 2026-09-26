use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub const MAX_PASSAGES: usize = 8;
pub const MAX_CONTEXT_TOKENS: usize = 6_000;
const MAX_UNIT_CHARS: usize = 4_000;
const RRF_K: f64 = 60.0;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationType {
    Knowledge,
    Idea,
}

impl Default for InformationType {
    fn default() -> Self {
        Self::Knowledge
    }
}

impl InformationType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Knowledge => "knowledge",
            Self::Idea => "idea",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStatus {
    Current,
    Historical,
    Unverified,
    Deferred,
    Rejected,
}

impl KnowledgeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Historical => "historical",
            Self::Unverified => "unverified",
            Self::Deferred => "deferred",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Adopted,
    Superseded,
    Withdrawn,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchAspect {
    Applicability,
    Decision,
    Content,
    Exploration,
}

impl SearchAspect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applicability => "applicability",
            Self::Decision => "decision",
            Self::Content => "content",
            Self::Exploration => "exploration",
        }
    }

    fn weight(self) -> f64 {
        match self {
            Self::Applicability => 1.5,
            Self::Decision => 1.2,
            Self::Content => 1.0,
            Self::Exploration => 0.55,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct ApplicabilityMetadata {
    summary: Option<String>,
    representative_questions: Vec<String>,
    helps_with: Vec<String>,
    conditions: Vec<String>,
    exclusions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct DecisionMetadata {
    id: String,
    topic: Option<String>,
    status: DecisionStatus,
    #[serde(default)]
    confirmed_final: bool,
    successor_id: Option<String>,
    reason: Option<String>,
    #[serde(default)]
    conditions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct LlmWikiMetadata {
    schema: u32,
    document_id: Option<String>,
    source_revision: Option<String>,
    #[serde(default)]
    information_type: InformationType,
    status: Option<KnowledgeStatus>,
    applicability: Option<ApplicabilityMetadata>,
    #[serde(default)]
    decisions: Vec<DecisionMetadata>,
    decision: Option<DecisionMetadata>,
}

#[derive(Debug, Deserialize)]
struct FrontMatter {
    llm_wiki: Option<LlmWikiMetadata>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DecisionDescriptor {
    pub id: String,
    pub topic: Option<String>,
    pub status: DecisionStatus,
    pub confirmed_final: bool,
    pub successor_id: Option<String>,
    pub reason: Option<String>,
    pub conditions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SearchUnit {
    pub unit_id: String,
    pub document_id: String,
    pub path: String,
    pub source_revision: String,
    pub declared_revision: Option<String>,
    pub section: String,
    pub chunk_index: usize,
    pub chunk_count: usize,
    pub aspect: SearchAspect,
    pub information_type: InformationType,
    pub status: Option<KnowledgeStatus>,
    pub text: String,
    pub input_hash: String,
    pub conditions: Vec<String>,
    pub decision: Option<DecisionDescriptor>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ParsedDocument {
    pub document_id: String,
    pub path: String,
    pub source_revision: String,
    pub declared_revision: Option<String>,
    pub information_type: InformationType,
    pub status: Option<KnowledgeStatus>,
    pub units: Vec<SearchUnit>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RankedCandidate {
    pub unit: SearchUnit,
    pub lexical_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
    pub semantic_score: Option<f32>,
    pub fused_score: f64,
    pub historical_match: Option<String>,
    pub warning: Option<String>,
}

impl RankedCandidate {
    pub fn lexical(unit: SearchUnit, rank: usize) -> Self {
        Self {
            unit,
            lexical_rank: Some(rank),
            semantic_rank: None,
            semantic_score: None,
            fused_score: 0.0,
            historical_match: None,
            warning: None,
        }
    }

    pub fn semantic(unit: SearchUnit, rank: usize, score: f32) -> Self {
        Self {
            unit,
            lexical_rank: None,
            semantic_rank: Some(rank),
            semantic_score: Some(score),
            fused_score: 0.0,
            historical_match: None,
            warning: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SuccessorResolution {
    Current(String),
    Unresolved(String),
}

#[derive(Clone, Debug)]
pub struct EmbeddingIdentity<'a> {
    pub unit_id: &'a str,
    pub source_revision: &'a str,
    pub input_hash: &'a str,
    pub model_id: &'a str,
    pub model_version: &'a str,
    pub dimensions: usize,
}

pub fn embedding_is_compatible(
    stored: &EmbeddingIdentity<'_>,
    expected: &EmbeddingIdentity<'_>,
    vector_bytes: &[u8],
) -> bool {
    stored.unit_id == expected.unit_id
        && stored.source_revision == expected.source_revision
        && stored.input_hash == expected.input_hash
        && stored.model_id == expected.model_id
        && stored.model_version == expected.model_version
        && stored.dimensions == expected.dimensions
        && stored.dimensions.checked_mul(4) == Some(vector_bytes.len())
}

pub fn parse_document(path: &str, markdown: &str) -> ParsedDocument {
    let source_revision = sha256(markdown.as_bytes());
    parse_document_at_revision(path, markdown, source_revision)
}

pub fn parse_document_at_revision(
    path: &str,
    markdown: &str,
    source_revision: String,
) -> ParsedDocument {
    let normalized_path = path.replace('\\', "/");
    let (front_matter, body, mut warnings) = split_front_matter(markdown);
    let metadata = front_matter.and_then(|yaml| match serde_yaml::from_str::<FrontMatter>(yaml) {
        Ok(front) => front.llm_wiki.and_then(|metadata| {
            if metadata.schema == 1 {
                Some(metadata)
            } else {
                warnings.push(format!(
                    "Unsupported llm_wiki metadata schema {}",
                    metadata.schema
                ));
                None
            }
        }),
        Err(_) => {
            warnings.push("Invalid llm_wiki YAML metadata; indexed body only".into());
            None
        }
    });
    let document_id = metadata
        .as_ref()
        .and_then(|value| nonempty(value.document_id.as_deref()))
        .map(str::to_owned)
        .unwrap_or_else(|| format!("path:{normalized_path}"));
    let declared_revision = metadata
        .as_ref()
        .and_then(|value| nonempty(value.source_revision.as_deref()))
        .map(str::to_owned);
    let information_type = metadata
        .as_ref()
        .map(|value| value.information_type)
        .unwrap_or_default();
    let status = metadata.as_ref().and_then(|value| value.status);
    let mut units = Vec::new();

    if let Some(applicability) = metadata
        .as_ref()
        .and_then(|value| value.applicability.as_ref())
    {
        push_optional_unit(
            &mut units,
            &document_id,
            &normalized_path,
            &source_revision,
            declared_revision.as_deref(),
            information_type,
            status,
            "Applicability",
            applicability.summary.as_deref(),
            SearchAspect::Applicability,
            applicability.conditions.clone(),
            None,
        );
        push_list_unit(
            &mut units,
            &document_id,
            &normalized_path,
            &source_revision,
            declared_revision.as_deref(),
            information_type,
            status,
            "Applicability > Representative questions",
            &applicability.representative_questions,
            applicability.conditions.clone(),
        );
        push_list_unit(
            &mut units,
            &document_id,
            &normalized_path,
            &source_revision,
            declared_revision.as_deref(),
            information_type,
            status,
            "Applicability > Helps with",
            &applicability.helps_with,
            applicability.conditions.clone(),
        );
        push_list_unit(
            &mut units,
            &document_id,
            &normalized_path,
            &source_revision,
            declared_revision.as_deref(),
            information_type,
            status,
            "Applicability > Conditions",
            &applicability.conditions,
            applicability.conditions.clone(),
        );
        push_list_unit(
            &mut units,
            &document_id,
            &normalized_path,
            &source_revision,
            declared_revision.as_deref(),
            information_type,
            status,
            "Applicability > Exclusions",
            &applicability.exclusions,
            applicability.conditions.clone(),
        );
    }

    let decisions = metadata
        .as_ref()
        .map(|value| {
            value
                .decisions
                .iter()
                .chain(value.decision.iter())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for decision in decisions {
        if decision.id.trim().is_empty() {
            warnings.push("Decision metadata has no stable id; decision unit omitted".into());
        } else {
            let descriptor = DecisionDescriptor {
                id: decision.id.trim().to_owned(),
                topic: decision
                    .topic
                    .as_deref()
                    .and_then(|value| nonempty(Some(value)))
                    .map(str::to_owned),
                status: decision.status,
                confirmed_final: decision.confirmed_final,
                successor_id: decision
                    .successor_id
                    .as_deref()
                    .and_then(|value| nonempty(Some(value)))
                    .map(str::to_owned),
                reason: decision
                    .reason
                    .as_deref()
                    .and_then(|value| nonempty(Some(value)))
                    .map(str::to_owned),
                conditions: clean_list(&decision.conditions),
            };
            let text = [
                Some(descriptor.id.as_str()),
                descriptor.topic.as_deref(),
                descriptor.reason.as_deref(),
                (!descriptor.conditions.is_empty())
                    .then(|| descriptor.conditions.join("\n"))
                    .as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
            push_unit(
                &mut units,
                &document_id,
                &normalized_path,
                &source_revision,
                declared_revision.as_deref(),
                information_type,
                status,
                &format!("Decision > {}", descriptor.id),
                &text,
                SearchAspect::Decision,
                descriptor.conditions.clone(),
                Some(descriptor),
            );
        }
    }

    let sections = markdown_sections(body);
    let mut section_occurrences = HashMap::new();
    for (section, text) in sections {
        let occurrence = section_occurrences
            .entry(section.clone())
            .or_insert(0_usize);
        *occurrence += 1;
        let section_locator = if *occurrence == 1 {
            section.clone()
        } else {
            format!("{section} [{}]", *occurrence)
        };
        let aspect = if is_applicability_heading(&section) {
            SearchAspect::Applicability
        } else if information_type == InformationType::Idea
            || matches!(
                status,
                Some(
                    KnowledgeStatus::Unverified
                        | KnowledgeStatus::Deferred
                        | KnowledgeStatus::Rejected
                )
            )
        {
            SearchAspect::Exploration
        } else {
            SearchAspect::Content
        };
        push_unit(
            &mut units,
            &document_id,
            &normalized_path,
            &source_revision,
            declared_revision.as_deref(),
            information_type,
            status,
            &section_locator,
            &text,
            aspect,
            Vec::new(),
            None,
        );
    }

    if units.is_empty() && !body.trim().is_empty() {
        push_unit(
            &mut units,
            &document_id,
            &normalized_path,
            &source_revision,
            declared_revision.as_deref(),
            information_type,
            status,
            "Document",
            body,
            if information_type == InformationType::Idea {
                SearchAspect::Exploration
            } else {
                SearchAspect::Content
            },
            Vec::new(),
            None,
        );
    }

    for unit in &mut units {
        unit.warnings = warnings.clone();
    }
    ParsedDocument {
        document_id,
        path: normalized_path,
        source_revision,
        declared_revision,
        information_type,
        status,
        units,
        warnings,
    }
}

pub fn fuse_candidates(
    lexical: impl IntoIterator<Item = RankedCandidate>,
    semantic: impl IntoIterator<Item = RankedCandidate>,
    limit: usize,
    offset: usize,
) -> Vec<RankedCandidate> {
    let mut merged: HashMap<String, RankedCandidate> = HashMap::new();
    for candidate in lexical.into_iter().chain(semantic) {
        let entry = merged
            .entry(candidate.unit.unit_id.clone())
            .or_insert_with(|| candidate.clone());
        if candidate.lexical_rank.is_some() {
            entry.lexical_rank = candidate.lexical_rank;
        }
        if candidate.semantic_rank.is_some() {
            entry.semantic_rank = candidate.semantic_rank;
            entry.semantic_score = candidate.semantic_score;
        }
    }
    let mut ranked = merged.into_values().collect::<Vec<_>>();
    for candidate in &mut ranked {
        let lexical = candidate
            .lexical_rank
            .map(|rank| 1.0 / (RRF_K + rank as f64))
            .unwrap_or(0.0);
        let semantic = candidate
            .semantic_rank
            .map(|rank| 1.0 / (RRF_K + rank as f64))
            .unwrap_or(0.0);
        candidate.fused_score = (lexical + semantic) * candidate.unit.aspect.weight();
    }
    ranked.sort_by(|left, right| {
        right
            .fused_score
            .partial_cmp(&left.fused_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.unit.document_id.cmp(&right.unit.document_id))
            .then_with(|| left.unit.section.cmp(&right.unit.section))
            .then_with(|| left.unit.unit_id.cmp(&right.unit.unit_id))
    });
    let mut seen_documents = HashSet::new();
    let mut tokens = 0_usize;
    ranked
        .into_iter()
        .filter(|candidate| seen_documents.insert(candidate.unit.document_id.clone()))
        .skip(offset)
        .take(limit)
        .take_while(|candidate| {
            let estimate = candidate.unit.text.chars().count().div_ceil(4);
            if tokens + estimate > MAX_CONTEXT_TOKENS {
                false
            } else {
                tokens += estimate;
                true
            }
        })
        .collect()
}

pub fn resolve_successor(
    matched: &SearchUnit,
    units: &[SearchUnit],
    query_conditions: &[String],
) -> SuccessorResolution {
    let Some(decision) = matched.decision.as_ref() else {
        return SuccessorResolution::Current(matched.unit_id.clone());
    };
    if decision.status != DecisionStatus::Superseded {
        return if decision.status == DecisionStatus::Adopted
            && decision.confirmed_final
            && conditions_compatible(&decision.conditions, query_conditions)
        {
            SuccessorResolution::Current(matched.unit_id.clone())
        } else {
            SuccessorResolution::Unresolved(
                "matched decision is not confirmed final or its conditions are unknown".into(),
            )
        };
    }
    let lookup = units
        .iter()
        .filter_map(|unit| {
            unit.decision
                .as_ref()
                .map(|value| (value.id.as_str(), unit))
        })
        .collect::<HashMap<_, _>>();
    let mut next = decision.successor_id.as_deref();
    let mut visited = HashSet::from([decision.id.as_str()]);
    while let Some(id) = next {
        if !visited.insert(id) {
            return SuccessorResolution::Unresolved("decision successor cycle".into());
        }
        let Some(unit) = lookup.get(id).copied() else {
            return SuccessorResolution::Unresolved("decision successor is missing".into());
        };
        let Some(candidate) = unit.decision.as_ref() else {
            return SuccessorResolution::Unresolved("decision successor is invalid".into());
        };
        match candidate.status {
            DecisionStatus::Superseded => next = candidate.successor_id.as_deref(),
            DecisionStatus::Adopted if candidate.confirmed_final => {
                if conditions_compatible(&candidate.conditions, query_conditions) {
                    return SuccessorResolution::Current(unit.unit_id.clone());
                }
                return SuccessorResolution::Unresolved("successor conditions differ".into());
            }
            DecisionStatus::Adopted | DecisionStatus::Withdrawn | DecisionStatus::Unresolved => {
                return SuccessorResolution::Unresolved(
                    "decision successor is not confirmed current".into(),
                );
            }
        }
    }
    SuccessorResolution::Unresolved("decision successor is missing".into())
}

pub fn is_indexable_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    !normalized.split('/').any(|part| {
        matches!(
            part,
            "Translations" | ".llm-wiki-recovery" | ".llm-wiki-withdrawn"
        )
    })
}

fn conditions_compatible(required: &[String], query: &[String]) -> bool {
    if required.is_empty() {
        return true;
    }
    if query.is_empty() {
        return false;
    }
    let query = query
        .iter()
        .map(|value| value.trim().to_lowercase())
        .collect::<HashSet<_>>();
    required
        .iter()
        .all(|value| query.contains(&value.trim().to_lowercase()))
}

fn split_front_matter(markdown: &str) -> (Option<&str>, &str, Vec<String>) {
    let normalized = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    if !normalized.starts_with("---\n") && !normalized.starts_with("---\r\n") {
        return (None, normalized, Vec::new());
    }
    let mut offset = 0;
    let mut lines = normalized.split_inclusive('\n');
    if let Some(first) = lines.next() {
        offset += first.len();
    }
    let yaml_start = offset;
    for line in lines {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            let yaml = &normalized[yaml_start..offset];
            let body = &normalized[offset + line.len()..];
            return (Some(yaml), body, Vec::new());
        }
        offset += line.len();
    }
    (
        None,
        normalized,
        vec!["Unclosed YAML front matter; indexed body only".into()],
    )
}

fn markdown_sections(body: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut heading = "Document".to_owned();
    let mut content = Vec::new();
    let mut fenced = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            content.push(line);
            continue;
        }
        if !fenced {
            let hashes = trimmed.chars().take_while(|value| *value == '#').count();
            if (1..=6).contains(&hashes)
                && trimmed.chars().nth(hashes).is_some_and(char::is_whitespace)
            {
                push_section(&mut sections, &heading, &content);
                heading = trimmed[hashes..].trim().to_owned();
                content.clear();
                continue;
            }
        }
        content.push(line);
    }
    push_section(&mut sections, &heading, &content);
    sections
}

fn push_section(sections: &mut Vec<(String, String)>, heading: &str, lines: &[&str]) {
    let text = lines.join("\n").trim().to_owned();
    if !text.is_empty() {
        sections.push((heading.to_owned(), text));
    }
}

fn is_applicability_heading(heading: &str) -> bool {
    let normalized = heading.trim().to_lowercase().replace(['-', '_'], " ");
    [
        "applicability",
        "when to use",
        "when to reuse",
        "reuse guidance",
        "representative questions",
        "helps with",
        "conditions",
        "exclusions",
        "적용",
        "적용성",
        "언제 사용",
        "재사용하기 좋은 경우",
        "참조하기 좋은 경우",
        "대표 질문",
        "도움이 되는 경우",
        "조건",
        "제외",
    ]
    .iter()
    .any(|candidate| normalized == *candidate || normalized.starts_with(&format!("{candidate} ")))
}

#[allow(clippy::too_many_arguments)]
fn push_list_unit(
    units: &mut Vec<SearchUnit>,
    document_id: &str,
    path: &str,
    source_revision: &str,
    declared_revision: Option<&str>,
    information_type: InformationType,
    status: Option<KnowledgeStatus>,
    section: &str,
    values: &[String],
    conditions: Vec<String>,
) {
    let text = clean_list(values).join("\n");
    push_optional_unit(
        units,
        document_id,
        path,
        source_revision,
        declared_revision,
        information_type,
        status,
        section,
        Some(&text),
        SearchAspect::Applicability,
        conditions,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_optional_unit(
    units: &mut Vec<SearchUnit>,
    document_id: &str,
    path: &str,
    source_revision: &str,
    declared_revision: Option<&str>,
    information_type: InformationType,
    status: Option<KnowledgeStatus>,
    section: &str,
    text: Option<&str>,
    aspect: SearchAspect,
    conditions: Vec<String>,
    decision: Option<DecisionDescriptor>,
) {
    if let Some(text) = text.and_then(|value| nonempty(Some(value))) {
        push_unit(
            units,
            document_id,
            path,
            source_revision,
            declared_revision,
            information_type,
            status,
            section,
            text,
            aspect,
            conditions,
            decision,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_unit(
    units: &mut Vec<SearchUnit>,
    document_id: &str,
    path: &str,
    source_revision: &str,
    declared_revision: Option<&str>,
    information_type: InformationType,
    status: Option<KnowledgeStatus>,
    section: &str,
    text: &str,
    aspect: SearchAspect,
    conditions: Vec<String>,
    decision: Option<DecisionDescriptor>,
) {
    let text = text.trim().chars().collect::<Vec<_>>();
    if text.is_empty() {
        return;
    }
    let normalized_section = section.split_whitespace().collect::<Vec<_>>().join(" ");
    let chunk_count = text.len().div_ceil(MAX_UNIT_CHARS);
    for (chunk_index, chunk) in text.chunks(MAX_UNIT_CHARS).enumerate() {
        let text = chunk.iter().collect::<String>();
        let unit_id = sha256(
            format!(
                "{document_id}\0{}\0{normalized_section}\0{chunk_index}",
                aspect.as_str()
            )
            .as_bytes(),
        );
        let input_hash = sha256(text.as_bytes());
        if units
            .iter()
            .any(|unit| unit.unit_id == unit_id && unit.input_hash == input_hash)
        {
            continue;
        }
        units.push(SearchUnit {
            unit_id,
            document_id: document_id.to_owned(),
            path: path.to_owned(),
            source_revision: source_revision.to_owned(),
            declared_revision: declared_revision.map(str::to_owned),
            section: normalized_section.clone(),
            chunk_index,
            chunk_count,
            aspect,
            information_type,
            status,
            text,
            input_hash,
            conditions: clean_list(&conditions),
            decision: decision.clone(),
            warnings: Vec::new(),
        });
    }
}

fn clean_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| nonempty(Some(value)).map(str::to_owned))
        .collect()
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn sha256(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision_markdown(
        id: &str,
        status: &str,
        successor: Option<&str>,
        finality: bool,
        conditions: &[&str],
    ) -> String {
        let successor = successor
            .map(|value| format!("    successor_id: {value}\n"))
            .unwrap_or_default();
        let conditions = conditions
            .iter()
            .map(|value| format!("      - {value}\n"))
            .collect::<String>();
        format!("---\nllm_wiki:\n  schema: 1\n  document_id: doc-{id}\n  decision:\n    id: {id}\n    status: {status}\n    confirmed_final: {finality}\n{successor}    conditions:\n{conditions}---\n# Decision\nSaved evidence.\n")
    }

    #[test]
    fn parses_standard_yaml_and_explicit_aspects() {
        let markdown = r#"---
title: Existing key remains compatible
llm_wiki:
  schema: 1
  document_id: stable-doc
  source_revision: declared-only
  information_type: idea
  status: unverified
  applicability:
    representative_questions:
      - How do I retrieve by use?
    helps_with: [retrieval, resumption]
    conditions: [local Vault]
  decision:
    id: d1
    status: adopted
    confirmed_final: false
---
# 내용
본문입니다.
"#;
        let parsed = parse_document("Ideas/example.md", markdown);
        assert_eq!(parsed.document_id, "stable-doc");
        assert_eq!(parsed.declared_revision.as_deref(), Some("declared-only"));
        assert_ne!(parsed.source_revision, "declared-only");
        assert!(parsed
            .units
            .iter()
            .any(|unit| unit.aspect == SearchAspect::Applicability));
        assert!(parsed
            .units
            .iter()
            .any(|unit| unit.aspect == SearchAspect::Decision
                && !unit.decision.as_ref().unwrap().confirmed_final));
        assert!(parsed
            .units
            .iter()
            .any(|unit| unit.aspect == SearchAspect::Exploration));
    }

    #[test]
    fn malformed_metadata_keeps_body_without_inventing_decisions() {
        let parsed = parse_document(
            "note.md",
            "---\nllm_wiki: [invalid\n---\n# Body\nStill searchable",
        );
        assert!(!parsed.warnings.is_empty());
        assert!(parsed
            .units
            .iter()
            .any(|unit| unit.text.contains("Still searchable")));
        assert!(parsed.units.iter().all(|unit| unit.decision.is_none()));
    }

    #[test]
    fn preserves_multiple_topic_decisions_without_promoting_siblings() {
        let parsed = parse_document(
            "multi.md",
            "---\nllm_wiki:\n  schema: 1\n  decisions:\n    - id: storage\n      topic: persistence\n      status: adopted\n      confirmed_final: true\n    - id: ranking\n      topic: retrieval\n      status: unresolved\n      confirmed_final: false\n---\n# Body\nShared document\n",
        );
        let decisions = parsed
            .units
            .iter()
            .filter_map(|unit| unit.decision.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].topic.as_deref(), Some("persistence"));
        assert!(decisions[0].confirmed_final);
        assert_eq!(decisions[1].topic.as_deref(), Some("retrieval"));
        assert!(!decisions[1].confirmed_final);
    }

    #[test]
    fn recognizes_korean_heading_but_not_fenced_heading() {
        let parsed = parse_document(
            "note.md",
            "# 조건\n오프라인에서 사용\n```md\n# Applicability\nnot a heading\n```\n# Notes\nOther",
        );
        assert!(parsed.units.iter().any(
            |unit| unit.aspect == SearchAspect::Applicability && unit.text.contains("오프라인")
        ));
        assert!(!parsed
            .units
            .iter()
            .any(|unit| unit.section == "Applicability"));
    }

    #[test]
    fn recognizes_explicit_reuse_headings_only() {
        for heading in [
            "When to reuse",
            "Reuse guidance",
            "재사용하기 좋은 경우",
            "참조하기 좋은 경우",
        ] {
            let parsed = parse_document("note.md", &format!("# {heading}\nBounded guidance"));
            assert_eq!(parsed.units[0].aspect, SearchAspect::Applicability);
        }
        let parsed = parse_document("note.md", "# Reuse\nToo broad to classify");
        assert_eq!(parsed.units[0].aspect, SearchAspect::Content);
    }

    #[test]
    fn chunks_long_sections_and_disambiguates_repeated_headings() {
        let long = "a".repeat(MAX_UNIT_CHARS + 25);
        let parsed = parse_document(
            "long.md",
            &format!("# Notes\n{long}\n# Notes\nsecond occurrence"),
        );
        let first = parsed
            .units
            .iter()
            .filter(|unit| unit.section == "Notes")
            .collect::<Vec<_>>();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].chunk_count, 2);
        assert_eq!(first[1].chunk_index, 1);
        assert!(parsed
            .units
            .iter()
            .any(|unit| unit.section == "Notes [2]" && unit.text == "second occurrence"));
        let unique = parsed
            .units
            .iter()
            .map(|unit| unit.unit_id.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(unique.len(), parsed.units.len());
    }

    #[test]
    fn fusion_accepts_independent_semantic_candidate_and_weights_applicability() {
        let applicable = parse_document("a.md", "# Applicability\nNo shared keyword")
            .units
            .remove(0);
        let content = parse_document("b.md", "# Body\nExact keyword")
            .units
            .remove(0);
        let ranked = fuse_candidates(
            [RankedCandidate::lexical(content, 1)],
            [RankedCandidate::semantic(applicable.clone(), 1, 0.9)],
            8,
            0,
        );
        assert_eq!(ranked[0].unit.document_id, applicable.document_id);
    }

    #[test]
    fn exploration_is_qualified_independent_of_folder() {
        let idea = parse_document("Knowledge/idea.md", "---\nllm_wiki:\n  schema: 1\n  information_type: idea\n  status: unverified\n---\n# Option\nMaybe").units.remove(0);
        let final_doc = parse_document("Ideas/final.md", "# Applicability\nUse this")
            .units
            .remove(0);
        let ranked = fuse_candidates(
            [
                RankedCandidate::lexical(idea, 1),
                RankedCandidate::lexical(final_doc.clone(), 2),
            ],
            [],
            8,
            0,
        );
        assert_eq!(ranked[0].unit.document_id, final_doc.document_id);
    }

    #[test]
    fn successor_resolution_handles_current_missing_cycle_and_conditions() {
        let old = parse_document(
            "old.md",
            &decision_markdown("old", "superseded", Some("new"), false, &[]),
        )
        .units
        .into_iter()
        .find(|unit| unit.decision.is_some())
        .unwrap();
        let new = parse_document(
            "new.md",
            &decision_markdown("new", "adopted", None, true, &["offline"]),
        )
        .units
        .into_iter()
        .find(|unit| unit.decision.is_some())
        .unwrap();
        assert_eq!(
            resolve_successor(&old, &[old.clone(), new.clone()], &["offline".into()]),
            SuccessorResolution::Current(new.unit_id.clone())
        );
        assert!(matches!(
            resolve_successor(&old, &[old.clone()], &[]),
            SuccessorResolution::Unresolved(_)
        ));
        assert!(matches!(
            resolve_successor(&old, &[old.clone(), new.clone()], &["cloud".into()]),
            SuccessorResolution::Unresolved(_)
        ));
        let cycle = parse_document(
            "cycle.md",
            &decision_markdown("new", "superseded", Some("old"), false, &[]),
        )
        .units
        .into_iter()
        .find(|unit| unit.decision.is_some())
        .unwrap();
        assert!(
            matches!(resolve_successor(&old, &[old.clone(), cycle], &[]), SuccessorResolution::Unresolved(message) if message.contains("cycle"))
        );
        assert!(matches!(
            resolve_successor(&new, &[new.clone()], &[]),
            SuccessorResolution::Unresolved(message) if message.contains("conditions")
        ));
        assert!(matches!(
            resolve_successor(&new, &[new.clone()], &["cloud".into()]),
            SuccessorResolution::Unresolved(message) if message.contains("conditions")
        ));
    }

    #[test]
    fn vector_identity_rejects_stale_or_malformed_vectors() {
        let expected = EmbeddingIdentity {
            unit_id: "u",
            source_revision: "r2",
            input_hash: "h",
            model_id: "m",
            model_version: "1",
            dimensions: 2,
        };
        let stale = EmbeddingIdentity {
            source_revision: "r1",
            ..expected.clone()
        };
        assert!(!embedding_is_compatible(&stale, &expected, &[0; 8]));
        assert!(!embedding_is_compatible(&expected, &expected, &[0; 4]));
        assert!(embedding_is_compatible(&expected, &expected, &[0; 8]));
    }

    #[test]
    fn internal_withdrawal_paths_are_not_indexed() {
        assert!(!is_indexable_path(".llm-wiki-withdrawn/old.md"));
        assert!(!is_indexable_path("nested/.llm-wiki-recovery/old.md"));
        assert!(is_indexable_path(".notes/useful.md"));
    }
}
