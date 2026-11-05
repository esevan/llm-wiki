//! Fill optional fields without manufacturing evidence. Malformed Knowledge
//! claim metadata is preserved diagnostically instead of discarding prose.
use super::PromptId;
use serde_json::{json, Value};

fn defaults(value: &mut Value, fields: &[(&str, Value)]) {
    if let Some(object) = value.as_object_mut() {
        for (key, empty) in fields {
            if !object.contains_key(*key) {
                object.insert((*key).into(), empty.clone());
            }
        }
    }
}

fn normalize_knowledge_claims(value: &mut Value) {
    // Never erase or replace malformed quality assessments while repairing
    // optional claim metadata. The provider schema must still reject them.
    if value.get("qualityFindings").is_some_and(|findings| !findings.is_array()) {
        return;
    }
    let mut warnings = Vec::new();
    if let Some(article) = value.get_mut("article").and_then(Value::as_object_mut) {
        for key in ["claimBindings", "assumptionBindings", "finalOutcomes"] {
            let Some(raw) = article.remove(key) else { continue; };
            let candidates = raw.as_array().cloned().unwrap_or_else(|| vec![raw]);
            let kept = candidates.into_iter().filter(|item| {
                let valid = match key {
                    "claimBindings" => serde_json::from_value::<crate::native::knowledge_distillation::ClaimBinding>(item.clone()).is_ok(),
                    "assumptionBindings" => serde_json::from_value::<crate::native::knowledge_distillation::AssumptionBinding>(item.clone()).is_ok(),
                    _ => serde_json::from_value::<crate::native::knowledge_distillation::FinalOutcome>(item.clone()).is_ok(),
                };
                if !valid { warnings.push(json!({"kind":"missing_reference","severity":"warning","locator":format!("article.{key}"),"sourceRefs":[],"message":format!("Unusable optional metadata retained for review: {item}")})); }
                valid
            }).collect::<Vec<_>>();
            article.insert(key.into(), json!(kept));
        }
    }
    if !warnings.is_empty() {
        let mut findings = value.get("qualityFindings").and_then(Value::as_array).cloned().unwrap_or_default();
        findings.extend(warnings);
        value["qualityFindings"] = json!(findings);
    }
}

// Narrow pre-schema repair: only optional Knowledge claim metadata, never
// article prose, quality assessments, source authority or other prompt kinds.
pub(super) fn normalize_knowledge_metadata(id: PromptId, value: &mut Value) {
    if id == PromptId::KnowledgeDraft { normalize_knowledge_claims(value); }
    if id == PromptId::KnowledgeRevisionChat {
        if let Some(candidate) = value.get_mut("candidate").filter(|v| v.is_object()) { normalize_knowledge_claims(candidate); }
    }
}

pub(crate) fn normalize_prompt_output(id: PromptId, value: &mut Value) {
    normalize_knowledge_metadata(id, value);
    match id {
        PromptId::WorkPreviewReply => defaults(value, &[("usedFindingIds", json!([]))]),
        PromptId::WorkPreviewPlanner | PromptId::WorkPreviewFinalizer => {
            crate::native::reference_aware_workbench::normalize_response(id, value, &Value::Null);
        }
        PromptId::KnowledgeDraft => {
            defaults(
                value,
                &[("ideas", json!([])), ("qualityFindings", json!([]))],
            );
            if let Some(article) = value.get_mut("article") {
                defaults(
                    article,
                    &[
                        ("type", json!("")),
                        ("title", json!("")),
                        ("applicability", json!({})),
                        ("finalOutcomes", json!([])),
                        ("claimBindings", json!([])),
                        ("assumptionBindings", json!([])),
                    ],
                );
                if let Some(applicability) = article.get_mut("applicability") {
                    defaults(
                        applicability,
                        &[
                            ("summary", json!("")),
                            ("representativeQuestions", json!([])),
                            ("helpsWith", json!([])),
                            ("conditions", json!([])),
                            ("exclusions", json!([])),
                        ],
                    );
                }
            }
            if let Some(ideas) = value.get_mut("ideas").and_then(Value::as_array_mut) {
                for idea in ideas {
                    defaults(idea, &[("relatedTopicKeys", json!([]))]);
                }
            }
            if let Some(findings) = value
                .get_mut("qualityFindings")
                .and_then(Value::as_array_mut)
            {
                for finding in findings {
                    defaults(finding, &[("message", json!(""))]);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_foundation::validate_prompt_output;

    fn preview() -> Value {
        json!({"preview":{"description":"Keep approval before export","goal":"Review CSV offline"},
            "optionalInvestigation":{"needed":false,"reason":"No remaining question"}})
    }

    #[test]
    fn optional_knowledge_claim_format_does_not_discard_experience_prose() {
        for id in [PromptId::KnowledgeDraft, PromptId::KnowledgeRevisionChat] {
            let candidate = json!({"article":{"type":"concept","title":"My experience","bodyMarkdown":"I tried the export, chose the local option, and left its outcome unverified.","applicability":{"summary":"","representativeQuestions":[],"helpsWith":[]},"claimBindings":[{"claimId":"incomplete"}],"assumptionBindings":null,"finalOutcomes":"malformed"},"ideas":[],"qualityFindings":[]});
            let mut value = if id == PromptId::KnowledgeDraft {candidate.clone()} else {json!({"message":"A grounded experience essay","missingEvidence":[],"candidate":candidate})};
            normalize_prompt_output(id, &mut value);
            let result = if id == PromptId::KnowledgeDraft {&value} else {&value["candidate"]};
            assert_eq!(result["article"]["bodyMarkdown"],"I tried the export, chose the local option, and left its outcome unverified.");
            assert_eq!(result["article"]["claimBindings"],json!([]));
            assert_eq!(result["article"]["finalOutcomes"],json!([]));
            assert!(result["qualityFindings"].as_array().unwrap().iter().all(|finding| finding["severity"]=="warning"));
            assert!(result["qualityFindings"].to_string().contains("incomplete"));
            super::super::validate_prompt_output(id, &value).unwrap();
        }
    }

    #[test]
    fn knowledge_prompts_request_detailed_grounded_first_person_essays() {
        for id in [PromptId::KnowledgeDraft, PromptId::KnowledgeRevisionChat] {
            let text=super::super::prompt_definition(id).instructions;
            assert!(text.contains("first-person experience essay"));
            assert!(text.contains("decisions"));
            assert!(text.contains("optional"));
            assert!(text.contains("Do not invent") || text.contains("never invent"));
        }
    }

    #[test]
    fn absent_optional_fields_are_filled_without_changing_supplied_values() {
        let mut planner = json!({"retrieval":{"needed":false,"reason":"Only user context"}});
        normalize_prompt_output(PromptId::WorkPreviewPlanner, &mut planner);
        validate_prompt_output(PromptId::WorkPreviewPlanner, &planner).unwrap();
        assert_eq!(planner["retrieval"]["queries"], json!([]));
        assert_eq!(planner["retrieval"]["filters"], json!({}));
        let mut value = preview();
        value["preview"]["constraints"] = json!(["Approval before export"]);
        normalize_prompt_output(PromptId::WorkPreviewFinalizer, &mut value);
        validate_prompt_output(PromptId::WorkPreviewFinalizer, &value).unwrap();
        assert_eq!(
            value["preview"]["constraints"],
            json!(["Approval before export"])
        );
        assert_eq!(value["preview"]["background"], "");
    }

    #[test]
    fn missing_essential_values_wrong_types_and_nulls_remain_errors() {
        for (id, mut value) in [
            (
                PromptId::WorkPreviewPlanner,
                json!({"retrieval":{"needed":null,"reason":"No need"}}),
            ),
            (
                PromptId::WorkPreviewPlanner,
                json!({"retrieval":{"needed":false,"reason":"No need"},"preliminaryAssumptions":null}),
            ),
            (
                PromptId::WorkPreviewFinalizer,
                json!({"preview":{"description":"","goal":"Review"}}),
            ),
            (
                PromptId::WorkPreviewFinalizer,
                json!({"preview":{"description":"Review","goal":"Review","constraints":"wrong type"}}),
            ),
        ] {
            normalize_prompt_output(id, &mut value);
            assert!(
                validate_prompt_output(id, &value).is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn defaults_never_manufacture_reference_identity_or_source_assumption_support() {
        for ids in [None, Some(json!([])), Some(json!(["goal:missing"]))] {
            let mut value = preview();
            value["preview"]["assumptions"] =
                json!([{"id":"a","text":"Needs approval","basis":"source"}]);
            if let Some(ids) = ids {
                value["preview"]["assumptions"][0]["sourceClaimIds"] = ids;
            }
            normalize_prompt_output(PromptId::WorkPreviewFinalizer, &mut value);
            assert!(validate_prompt_output(PromptId::WorkPreviewFinalizer, &value).is_err());
        }
        let mut value = preview();
        value["claimSources"] =
            json!([{"claimId":"goal:0","documentId":"d","documentVersion":"v","role":"support"}]);
        normalize_prompt_output(PromptId::WorkPreviewFinalizer, &mut value);
        assert!(
            validate_prompt_output(PromptId::WorkPreviewFinalizer, &value)
                .unwrap_err()
                .to_string()
                .contains("invalid_reference_binding")
        );
    }

    #[test]
    #[ignore = "Synthetic local timing receipt; no network or model calls"]
    fn measure_optional_field_normalization_cost() {
        let cases = [
            (
                PromptId::WorkPreviewPlanner,
                json!({"retrieval":{"needed":false,"reason":"No lookup"}}),
            ),
            (PromptId::WorkPreviewFinalizer, preview()),
            (
                PromptId::KnowledgeDraft,
                json!({"article":{"type":"concept","title":"Manual evidence","bodyMarkdown":"Keep approval","applicability":{"summary":"Keep approval","representativeQuestions":["When?"],"helpsWith":["Keep approval"],"sourceRefs":[]}}}),
            ),
        ];
        for (id, seed) in cases {
            let mut total = std::time::Duration::ZERO;
            for _ in 0..20000 {
                let mut value = seed.clone();
                let start = std::time::Instant::now();
                normalize_prompt_output(id, std::hint::black_box(&mut value));
                total += start.elapsed();
                std::hint::black_box(value);
            }
            eprintln!(
                "normalization {}: 20000 iterations; mean {:.3} us (debug, clone excluded)",
                id.as_str(),
                total.as_secs_f64() * 1_000_000.0 / 20000.0
            );
        }
    }
}
