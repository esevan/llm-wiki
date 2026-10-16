//! Request-local preview references. Canonical source identities never come from the model.
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn fail(stage: &str) -> String {
    format!("invalid_reference_binding: {stage}")
}
fn tuple(reference: &Value) -> Result<(&str, &str, &str), String> {
    let document = reference["documentId"].as_str().filter(|s| !s.is_empty());
    let version = reference["documentVersion"]
        .as_str()
        .filter(|s| !s.is_empty());
    match (document, version, reference["section"].as_str()) {
        (Some(d), Some(v), Some(s)) => Ok((d, v, s)),
        _ => Err(fail("reference_catalogue: invalid source identity")),
    }
}
pub(crate) fn assign(references: &mut Vec<Value>) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    let request = uuid::Uuid::new_v4().simple().to_string();
    for reference in references.iter() {
        let identity = tuple(reference)?;
        if !seen.insert((
            identity.0.to_owned(),
            identity.1.to_owned(),
            identity.2.to_owned(),
        )) {
            continue;
        }
        let mut reference = reference.clone();
        reference["referenceId"] = json!(format!("ref_{request}_{}", unique.len()));
        unique.push(reference);
    }
    *references = unique;
    Ok(())
}
fn catalogue(context: &Value) -> Result<HashMap<&str, &Value>, String> {
    let mut result = HashMap::new();
    for reference in context["references"].as_array().into_iter().flatten() {
        tuple(reference)?;
        let id = reference["referenceId"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| fail("reference_catalogue: missing referenceId"))?;
        if result.insert(id, reference).is_some() {
            return Err(fail("reference_catalogue: duplicate referenceId"));
        }
    }
    Ok(result)
}
pub(crate) fn claim_schema(context: &Value) -> Result<Value, String> {
    let catalogue = catalogue(context)?;
    let mut ids: Vec<_> = catalogue.keys().copied().collect();
    ids.sort_unstable();
    let reference = if ids.is_empty() {
        json!({"type":"string"})
    } else {
        json!({"type":"string", "enum":ids})
    };
    let mut schema = json!({"type":"array", "items":{"type":"object", "additionalProperties":false,
        "required":["claimId","referenceId","role"], "properties":{
            "claimId":{"type":"string"}, "referenceId":reference, "role":{"type":"string"}}}});
    if catalogue.is_empty() {
        schema["maxItems"] = json!(0);
    }
    Ok(schema)
}
pub(crate) fn restore(context: &Value, output: &mut Value) -> Result<(), String> {
    let catalogue = catalogue(context)?;
    let Some(claims) = output.get_mut("claimSources") else {
        return Ok(());
    };
    let claims = claims
        .as_array_mut()
        .ok_or_else(|| fail("response_mapping: invalid claimSources"))?;
    let mut seen = HashSet::new();
    for claim in claims {
        let id = claim["referenceId"]
            .as_str()
            .ok_or_else(|| fail("response_mapping: missing referenceId"))?;
        let source = catalogue
            .get(id)
            .ok_or_else(|| fail("response_mapping: unknown referenceId"))?;
        let claim_id = claim["claimId"]
            .as_str()
            .ok_or_else(|| fail("response_mapping: missing claimId"))?;
        if !seen.insert((claim_id.to_owned(), id.to_owned())) {
            return Err(fail("response_mapping: duplicate claim reference"));
        }
        let object = claim
            .as_object_mut()
            .ok_or_else(|| fail("response_mapping: invalid claim"))?;
        object.remove("referenceId");
        for key in ["documentId", "documentVersion", "section"] {
            object.insert(key.into(), source[key].clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_foundation::{structured_output, PromptId};
    fn refs() -> Vec<Value> {
        vec![
            json!({"documentId":"doc","documentVersion":"v1","section":"a"}),
            json!({"documentId":"doc","documentVersion":"v2","section":"a"}),
            json!({"documentId":"doc","documentVersion":"v2","section":"b"}),
        ]
    }
    fn payload(claims: Value) -> Value {
        json!({"choices":[{"finish_reason":"stop","message":{"content":json!({"claimSources":claims}).to_string()}}]})
    }
    #[test]
    fn request_local_catalogue_preserves_versions_sections_and_rejects_cross_request_ids() {
        let mut first = refs();
        first.push(first[0].clone());
        assign(&mut first).unwrap();
        assert_eq!(first.len(), 3);
        let mut second = refs();
        assign(&mut second).unwrap();
        let context = json!({"references":first});
        let schema = claim_schema(&context).unwrap();
        assert_eq!(
            schema["items"]["properties"]["referenceId"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        let claims: Vec<_> = first.iter().enumerate().map(|(i,r)| json!({"claimId":format!("constraints:{i}"),"referenceId":r["referenceId"],"role":"constraint"})).collect();
        let output = structured_output::parse_completion(
            PromptId::WorkPreviewFinalizer,
            &context,
            &payload(json!(claims)),
        )
        .unwrap();
        for (claim, source) in output["claimSources"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&first)
        {
            for key in ["documentId", "documentVersion", "section"] {
                assert_eq!(claim[key], source[key]);
            }
            assert!(claim.get("referenceId").is_none());
        }
        let invalid =
            json!([{"claimId":"goal:x","referenceId":second[0]["referenceId"],"role":"source"}]);
        assert!(structured_output::parse_completion(
            PromptId::WorkPreviewFinalizer,
            &context,
            &payload(invalid)
        )
        .is_err());
    }
    #[test]
    fn malformed_duplicate_and_empty_bindings_fail_closed() {
        let mut references = refs();
        assign(&mut references).unwrap();
        let context = json!({"references":references});
        let good =
            json!({"claimId":"goal:x","referenceId":references[0]["referenceId"],"role":"source"});
        for bad in [
            json!({"claimId":"goal:x","role":"source"}),
            json!({"claimId":"goal:x","referenceId":null,"role":"source"}),
            json!({"claimId":"goal:x","referenceId":"unknown","role":"source"}),
            json!({"claimId":"goal:x","referenceId":references[0]["referenceId"],"documentId":"forged","role":"source"}),
        ] {
            assert!(structured_output::parse_completion(
                PromptId::WorkPreviewFinalizer,
                &context,
                &payload(json!([bad]))
            )
            .is_err());
        }
        assert!(structured_output::parse_completion(
            PromptId::WorkPreviewFinalizer,
            &context,
            &payload(json!([good, good]))
        )
        .unwrap_err()
        .contains("duplicate claim reference"));
        let mut another = good.clone();
        another["claimId"] = json!("background:y");
        assert!(structured_output::parse_completion(
            PromptId::WorkPreviewFinalizer,
            &context,
            &payload(json!([good, another]))
        )
        .is_ok());
        assert!(
            claim_schema(&json!({"references":[references[0],references[0]]}))
                .unwrap_err()
                .contains("duplicate referenceId")
        );
        let empty = claim_schema(&json!({"references":[]})).unwrap();
        assert_eq!(empty["maxItems"], 0);
        assert!(empty["items"]["properties"]["referenceId"]
            .get("enum")
            .is_none());
        assert!(structured_output::parse_completion(
            PromptId::WorkPreviewFinalizer,
            &json!({"references":[]}),
            &payload(json!([good]))
        )
        .is_err());
    }
    #[test]
    fn bounded_catalogue_cost_receipt() {
        let mut references: Vec<_> = (0..32).map(|i| json!({"documentId":format!("doc-{i}"),"documentVersion":"sha256:0123456789abcdef","section":"Heading","excerpt":"Evidence"})).collect();
        let before = serde_json::to_vec(&references).unwrap().len();
        assign(&mut references).unwrap();
        let after = serde_json::to_vec(&references).unwrap().len();
        let context = json!({"references":references});
        let start = std::time::Instant::now();
        let mut schema_bytes = 0;
        for _ in 0..100 {
            schema_bytes = serde_json::to_vec(&claim_schema(&context).unwrap())
                .unwrap()
                .len();
        }
        println!("32 references: catalogue ID overhead={} bytes; claim schema={} bytes; 100 schema builds={:?}; additional model calls=0",after-before,schema_bytes,start.elapsed());
        assert!(schema_bytes < 4096);
        assert!(after - before < 2048);
    }
}
