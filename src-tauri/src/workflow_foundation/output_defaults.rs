//! Fill absent optional fields only; never repair types or manufacture evidence.
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

pub(crate) fn normalize_prompt_output(id: PromptId, value: &mut Value) {
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
