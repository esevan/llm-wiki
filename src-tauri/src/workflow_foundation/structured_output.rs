//! One schema contract for provider requests and response type checks.
//! Semantic/source/approval checks remain in their owning validators.
use super::PromptId;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
fn text() -> Value {
    json!({"type":"string"})
}
fn capture_text(limit: usize) -> Value {
    json!({"type":"string","maxLength":limit,"description":format!("At most {limit} Unicode scalar values after trimming. This is a maximum, not a target length.")})
}
fn list(item: Value) -> Value {
    json!({"type":"array","items":item})
}
fn strings() -> Value {
    list(text())
}
fn nullable(value: Value) -> Value {
    json!({"anyOf":[value,{"type":"null"}]})
}
fn object(fields: &[(&str, Value)]) -> Value {
    let properties: Map<String, Value> = fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    json!({"type":"object","required":properties.keys().collect::<Vec<_>>(),"properties":properties,"additionalProperties":false})
}
fn localized(value: Value) -> Value {
    object(&[("ko", value.clone()), ("en", value)])
}
fn task_fields() -> Value {
    object(&[
        (
            "title",
            json!({"type":"string","minLength":1,"maxLength":70}),
        ),
        ("detail", text()),
        ("outcome", text()),
        ("scope", text()),
        ("nonGoals", text()),
        ("validationCriteria", text()),
    ])
}
// Strict JSON Schema cannot describe open-key objects. This explicit wire
// object is a lossless representation, decoded only in declared generic fields.
fn generic_value() -> Value {
    json!({"$ref":"#/$defs/JsonValue"})
}
fn generic_object() -> Value {
    object(&[(
        "_entries",
        list(object(&[("key", text()), ("value", generic_value())])),
    )])
}
fn legacy_proposals() -> Value {
    let mut patch = task_fields();
    for field in patch["properties"].as_object_mut().unwrap().values_mut() {
        *field = nullable(field.clone());
    }
    let task = task_fields();
    let mut subtask = task.clone();
    for (key, value) in [
        ("parentTaskId", text()),
        ("expectedTaskRevision", json!({"type":"integer"})),
        ("boundaryReason", text()),
    ] {
        subtask["properties"][key] = value;
    }
    let specs = [
        (
            "task_patch",
            object(&[
                ("patch", patch),
                ("expectedTaskRevision", json!({"type":"integer"})),
                ("hierarchyContextHash", nullable(text())),
            ]),
        ),
        ("new_task", task.clone()),
        ("subtask", subtask),
        (
            "problem_snapshot",
            object(&[
                ("statement", text()),
                ("detail", text()),
                ("category", text()),
                ("note", text()),
                ("problemId", nullable(text())),
            ]),
        ),
        (
            "task_problem_link",
            object(&[
                ("taskId", nullable(text())),
                ("problemId", text()),
                ("problemRevision", json!({"type":"integer"})),
                ("relationship", text()),
                ("note", text()),
            ]),
        ),
    ];
    let variants = specs
        .into_iter()
        .map(|(kind, payload)| {
            let mut fields = vec![
                ("id", text()),
                ("type", json!({"type":"string","enum":[kind]})),
                ("payload", payload),
            ];
            if matches!(kind, "task_patch" | "new_task" | "subtask") {
                fields.push(("localizedFields", localized(task.clone())));
            }
            object(&fields)
        })
        .collect::<Vec<_>>();
    object(&[("proposals", list(json!({"anyOf":variants})))])
}
fn generic_decode(value: &Value) -> Result<Value, String> {
    match value {
        Value::Array(a) => Ok(Value::Array(
            a.iter().map(generic_decode).collect::<Result<_, _>>()?,
        )),
        Value::Object(o) => {
            let entries = o
                .get("_entries")
                .and_then(Value::as_array)
                .ok_or("invalid structured JSON object wire format")?;
            let mut out = Map::new();
            for entry in entries {
                let fields = entry.as_object().ok_or("invalid structured JSON entry")?;
                if fields.len() != 2 || !fields.contains_key("key") || !fields.contains_key("value")
                {
                    return Err("structured JSON entry requires exactly key and value".into());
                }
                let key = entry["key"].as_str().ok_or("invalid structured JSON key")?;
                if out
                    .insert(key.into(), generic_decode(&entry["value"])?)
                    .is_some()
                {
                    return Err("duplicate structured JSON key".into());
                }
            }
            Ok(Value::Object(out))
        }
        _ => Ok(value.clone()),
    }
}
fn intent() -> Value {
    let reason = json!({"type":"string","minLength":1,"pattern":"\\S"});
    let queries = |min, max| {
        json!({"type":"array","minItems":min,"maxItems":max,
        "items":{"type":"string","minLength":1,"maxLength":500,"pattern":"\\S"}})
    };
    let branch = |needed: bool| {
        object(&[
            ("needed", json!({"type":"boolean","enum":[needed]})),
            ("reason", reason.clone()),
            (
                "queries",
                queries(if needed { 1 } else { 0 }, if needed { 4 } else { 0 }),
            ),
            (
                "aspects",
                json!({"type":"array","items":{"type":"string","enum":["content","applicability","decision","exploration"]}}),
            ),
            (
                "filters",
                object(&[
                    (
                        "informationTypes",
                        json!({"type":"array","maxItems":16,"items":{"type":"string"}}),
                    ),
                    (
                        "statuses",
                        json!({"type":"array","maxItems":16,"items":{"type":"string"}}),
                    ),
                ]),
            ),
            (
                "requery",
                if needed {
                    nullable(object(&[
                        ("reason", reason.clone()),
                        ("queries", queries(1, 2)),
                    ]))
                } else {
                    json!({"type":"null"})
                },
            ),
        ])
    };
    json!({"anyOf":[branch(false),branch(true)]})
}
fn derive<T: schemars::JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("schema serialization")
}
fn ids(context: &Value, key: &str) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    fn walk(v: &Value, key: &str, out: &mut BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(map) = o.get(key).and_then(Value::as_object) {
                    out.extend(map.keys().cloned());
                }
                for v in o.values() {
                    walk(v, key, out)
                }
            }
            Value::Array(a) => {
                for v in a {
                    walk(v, key, out)
                }
            }
            _ => {}
        }
    }
    walk(context, key, &mut result);
    result
}
/// Wire schemas are complete; legacy omission recovery still runs only after type checks.
pub(crate) fn schema(id: PromptId, context: &Value) -> Result<Value, String> {
    let value = match id {
        PromptId::WorkPreviewPlanner => object(&[
            ("retrieval", intent()),
            (
                "preliminaryAssumptions",
                list(object(&[
                    ("id", text()),
                    ("text", text()),
                    (
                        "basis",
                        json!({"type":"string","enum":["user_context","source","model_inference"]}),
                    ),
                ])),
            ),
        ]),
        PromptId::WorkPreviewFinalizer => {
            let mut fields = vec![
                (
                    "title",
                    json!({"type":"string","minLength":1,"maxLength":70}),
                ),
                ("description", text()),
                ("background", text()),
                ("goal", text()),
                ("scope", text()),
                ("nonGoals", text()),
            ];
            fields.extend([("constraints",strings()),("completionCriteria",strings()),("initialApproach",strings()),("assumptions",list(object(&[("id",text()),("text",text()),("basis",json!({"type":"string","enum":["user_context","source","model_inference"]})),("sourceClaimIds",strings())])))]);
            object(&[
                ("preview", object(&fields)),
                (
                    "claimSources",
                    super::preview_references::claim_schema(context)?,
                ),
                ("optionalInvestigation", intent()),
            ])
        }
        PromptId::WorkPreviewReply => object(&[("message", text()), ("usedFindingIds", strings())]),
        PromptId::RefinementResponse => object(&[("message", text())]),
        PromptId::WorkPreviewInvestigation => object(&[(
            "findings",
            list(object(&[
                (
                    "priority",
                    json!({"type":"string","enum":["critical","supporting","ancillary"]}),
                ),
                ("summary", text()),
                ("referenceIds", strings()),
            ])),
        )]),
        PromptId::KnowledgeRevisionChat => derive::<crate::native::knowledge_revision_chat::Reply>(),
        PromptId::KnowledgeDraft => {
            derive::<crate::native::knowledge_distillation::GenerationResult>()
        }
        PromptId::KnowledgeArchiveProposal => {
            derive::<crate::domain::knowledge_archive::Organization>()
        }
        PromptId::WorkLogDistillation
        | PromptId::RunReportDistillation
        | PromptId::TaskJourneyIncrement => {
            derive::<crate::application::task_distillation_service::DistillationResult>()
        }
        PromptId::ProblemEnrichment => object(&[
            ("normalized_problem", text()),
            ("pain", text()),
            ("non_goals", strings()),
            ("categories", strings()),
            ("importance_rationale", text()),
        ]),
        PromptId::ImageSummary => localized(object(&[("summary", text())])),
        PromptId::KnowledgeTranslation => object(&[("markdown", text())]),
        PromptId::DerivedTranslation => object(&[
            ("translation_needed", json!({"type":"boolean"})),
            (
                "source_locale",
                nullable(json!({"type":"string","enum":["ko","en"]})),
            ),
            ("ko", nullable(text())),
            ("en", nullable(text())),
        ]),
        PromptId::WorkflowDraftCapture => {
            localized(object(&[("title", text()), ("detail", text())]))
        }
        PromptId::WorkflowDraftProblem => object(&[
            ("validation_criteria", text()),
            (
                "ko",
                object(&[("title", text()), ("detail", text()), ("outcome", text())]),
            ),
            (
                "en",
                object(&[("title", text()), ("detail", text()), ("outcome", text())]),
            ),
        ]),
        PromptId::WorkflowRefinement => {
            fn problems(v: &Value) -> bool {
                v.get("entityType").and_then(Value::as_str) == Some("problems")
                    || v.as_object().is_some_and(|o| o.values().any(problems))
            }
            if problems(context) {
                localized(object(&[("title", text()), ("detail", text())]))
            } else {
                object(&[("title", text())])
            }
        }
        PromptId::TaskJourney => {
            let mut properties = Map::new();
            fn events(v: &Value, out: &mut Map<String, Value>) {
                if let Some(a) = v.get("events").and_then(Value::as_array) {
                    for e in a {
                        if let Some(id) = e["id"].as_str() {
                            out.insert(id.into(), localized(text()));
                        }
                    }
                }
                if let Some(o) = v.as_object() {
                    for child in o.values() {
                        events(child, out)
                    }
                }
            }
            events(context, &mut properties);
            object(&[
                (
                    "titles",
                    json!({"type":"object","properties":properties,"additionalProperties":false}),
                ),
                (
                    "relationships",
                    list(object(&[
                        ("from", text()),
                        ("to", text()),
                        (
                            "kind",
                            json!({"type":"string","enum":["supersedes","derived_from","depends_on"]}),
                        ),
                        ("rationale", text()),
                        (
                            "evidence",
                            list(object(&[("eventId", text()), ("quote", text())])),
                        ),
                    ])),
                ),
            ])
        }
        PromptId::LineageInference => object(&[
            (
                "claims",
                list(object(&[
                    ("text", text()),
                    ("evidence_ids", strings()),
                    ("claim_key", text()),
                    ("confidence", text()),
                ])),
            ),
            ("evidence_ids", strings()),
        ]),
        PromptId::WorkbenchOrganization => object(&[(
            "entries",
            list(object(&[
                ("entity_type", text()),
                ("entity_id", text()),
                ("category", text()),
                ("attention_rank", json!({"type":"integer"})),
                ("rationale", text()),
            ])),
        )]),
        PromptId::ConflictReview => object(&[
            (
                "status",
                json!({"type":"string","enum":["clear","findings","insufficient_evidence"]}),
            ),
            (
                "citations",
                list(object(&[("path", text()), ("excerpt", text())])),
            ),
            (
                "findings",
                list(object(&[
                    ("id", text()),
                    ("path", text()),
                    ("summary", text()),
                ])),
            ),
        ]),
        PromptId::CompletionReport => object(&[
            ("executive_summary_markdown", text()),
            ("report_body_markdown", text()),
        ]),
        PromptId::CaptureDistillation => {
            use crate::native::capture_distillation::{
                CONTENT_LIMIT, CONTEXT_LIMIT, REQUEST_COUNT_LIMIT, REQUEST_LIMIT, TITLE_LIMIT,
            };
            object(&[
                ("schemaVersion", json!({"type":"integer","enum":[1]})),
                ("title", capture_text(TITLE_LIMIT)),
                ("content", capture_text(CONTENT_LIMIT)),
                ("context", capture_text(CONTEXT_LIMIT)),
                (
                    "explicitRequests",
                    json!({"type":"array","maxItems":REQUEST_COUNT_LIMIT,
                    "description":format!("At most {REQUEST_COUNT_LIMIT} explicitly supported requests."),"items":capture_text(REQUEST_LIMIT)}),
                ),
                (
                    "grounding",
                    list(object(&[("field", text()), ("sourceRefs", strings())])),
                ),
            ])
        }
        PromptId::RefinementPreview => legacy_proposals(),
        PromptId::CompletionReview => object(&[
            ("resolution", text()),
            ("executive_summary", text()),
            ("what_changed", generic_value()),
            ("criteria_review", generic_value()),
            ("remaining_checklist", generic_value()),
            ("decision_rationale", text()),
            ("problem_recommendation", text()),
            ("capture_recommendation", text()),
        ]),
        PromptId::PublicationIndex => object(&[("entries", list(generic_value()))]),
        PromptId::SpeculativeSearch => object(&[("results", list(generic_value()))]),
        PromptId::UserGeneration => object(&[("proposal", generic_object())]),
    };
    let mut value = value;
    if matches!(
        id,
        PromptId::CompletionReview
            | PromptId::PublicationIndex
            | PromptId::SpeculativeSearch
            | PromptId::UserGeneration
    ) {
        value["$defs"]["JsonValue"] = json!({"anyOf":[{"type":"null"},{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"array","items":generic_value()},generic_object()]});
    }
    if id == PromptId::KnowledgeArchiveProposal {
        value["properties"]["outcome"]["enum"] =
            json!(["new", "update", "merge", "conflict", "supersede"]);
    }
    if id == PromptId::WorkPreviewFinalizer {
        for key in ["description", "background", "goal", "scope", "nonGoals"] {
            value["properties"]["preview"]["properties"][key]["maxLength"] = json!(12000);
        }
        for key in ["constraints", "completionCriteria", "initialApproach"] {
            value["properties"]["preview"]["properties"][key]["maxItems"] = json!(32);
            value["properties"]["preview"]["properties"][key]["items"]["maxLength"] = json!(12000);
        }
    }
    let targets = ids(context, "semanticTargetRevisions");
    strictify(&mut value, &targets)?;
    Ok(value)
}
fn strictify(value: &mut Value, targets: &BTreeSet<String>) -> Result<(), String> {
    if let Some(o) = value.as_object_mut() {
        o.remove("$schema");
        o.remove("default");
        o.remove("format");
        if o.get("type") == Some(&json!("object")) {
            if !o.contains_key("properties") {
                // The only exported dynamic map is expectedTargetRevisions. Null denotes
                // an unused known target on the wire, removed before canonical validation.
                if o.get("additionalProperties").is_some_and(Value::is_object) {
                    o.insert(
                        "properties".into(),
                        Value::Object(
                            targets
                                .iter()
                                .map(|id| (id.clone(), nullable(json!({"type":"integer"}))))
                                .collect(),
                        ),
                    );
                } else {
                    o.insert("properties".into(), json!({}));
                }
            }
            let required = o["properties"]
                .as_object()
                .unwrap()
                .keys()
                .map(|k| json!(k))
                .collect::<Vec<_>>();
            o.insert("required".into(), json!(required));
            o.insert("additionalProperties".into(), json!(false));
        }
        for (key, child) in o.iter_mut() {
            if key != "enum" && key != "required" {
                strictify(child, targets)?
            }
        }
    } else if let Some(a) = value.as_array_mut() {
        for child in a {
            strictify(child, targets)?
        }
    }
    Ok(())
}
// Some strict-schema providers/models reject maxLength/maxItems. Retain the
// limits in descriptions on the wire and enforce them locally for every provider.
fn capture_wire_schema(schema: &mut Value) {
    match schema {
        Value::Object(object) => {
            object.remove("maxLength");
            object.remove("maxItems");
            for value in object.values_mut() {
                capture_wire_schema(value);
            }
        }
        Value::Array(array) => {
            for value in array {
                capture_wire_schema(value);
            }
        }
        _ => (),
    }
}

pub(crate) fn request_body(
    id: PromptId,
    context: &Value,
    model: &str,
    mut content: Value,
) -> Result<Value, String> {
    let note=match id {
        PromptId::CompletionReview|PromptId::PublicationIndex|PromptId::SpeculativeSearch|PromptId::UserGeneration=>"\nStrict schema wire format: arbitrary JSON objects inside generic fields must be encoded as {\"_entries\":[{\"key\":\"original key\",\"value\":value}]}, recursively. Arrays/scalars keep their JSON types. This transport encoding is decoded losslessly before application validation. Do not invent extra content.",
        PromptId::WorkLogDistillation|PromptId::RunReportDistillation|PromptId::TaskJourneyIncrement=>"\nFor expectedTargetRevisions, the schema lists known target IDs. Return the exact revision for selected targetIds and null for every unused target. Null placeholders are removed before canonical validation.",
        PromptId::RefinementPreview=>"\nIn a sparse task_patch, use null for unchanged patch fields and unused optional IDs. They are omitted during transport decoding, preserving existing Task fields. Never use arrays for text fields.",
        _=>""
    };
    if !note.is_empty() {
        if let Some(text) = content.as_str() {
            content = json!(format!("{text}{note}"));
        } else if let Some(parts) = content.as_array_mut() {
            parts.push(json!({"type":"text","text":note}));
        }
    }
    let mut response_schema = schema(id, context)?;
    if id == PromptId::CaptureDistillation {
        capture_wire_schema(&mut response_schema);
    }
    Ok(
        json!({"model":model,"messages":[{"role":"user","content":content}],"stream":false,
 "response_format":{"type":"json_schema","json_schema":{"name":id.as_str(),"strict":true,"schema":response_schema}}}),
    )
}
pub(crate) fn http_error(status: reqwest::StatusCode) -> String {
    if matches!(status.as_u16(), 400 | 422) {
        format!("structured_output_request_rejected ({status}): provider/model must support strict response_format json_schema; no schema-free fallback or automatic retry")
    } else {
        format!("Provider request failed ({status})")
    }
}
/// No text-array coercion. Only null placeholders of the explicit dynamic-map
/// wire contract are decoded; all non-null values still face domain checks.
pub(crate) fn decode(id: PromptId, mut value: Value) -> Result<Value, String> {
    if matches!(
        id,
        PromptId::WorkLogDistillation
            | PromptId::RunReportDistillation
            | PromptId::TaskJourneyIncrement
    ) {
        if let Some(changes) = value["nodeChanges"].as_array_mut() {
            for change in changes {
                if let Some(map) = change["expectedTargetRevisions"].as_object_mut() {
                    map.retain(|_, v| !v.is_null());
                }
            }
        }
    }
    let generic_fields: &[&str] = match id {
        PromptId::UserGeneration => &["proposal"],
        PromptId::PublicationIndex => &["entries"],
        PromptId::SpeculativeSearch => &["results"],
        PromptId::CompletionReview => &["what_changed", "criteria_review", "remaining_checklist"],
        _ => &[],
    };
    for field in generic_fields {
        if let Some(v) = value.get_mut(*field) {
            *v = generic_decode(v)?;
        }
    }
    if id == PromptId::RefinementPreview {
        if let Some(proposals) = value["proposals"].as_array_mut() {
            for proposal in proposals {
                if let Some(payload) = proposal["payload"].as_object_mut() {
                    payload.retain(|_, v| !v.is_null());
                    if let Some(patch) = payload.get_mut("patch").and_then(Value::as_object_mut) {
                        patch.retain(|_, v| !v.is_null());
                    }
                }
            }
        }
    }
    Ok(value)
}

pub(crate) fn parse_completion(
    id: PromptId,
    context: &Value,
    payload: &Value,
) -> Result<Value, String> {
    let choice = &payload["choices"][0];
    if choice["message"]["refusal"]
        .as_str()
        .is_some_and(|s| !s.is_empty())
    {
        return Err("structured_output_refused: provider declined generation".into());
    }
    if choice["finish_reason"]
        .as_str()
        .is_some_and(|s| s != "stop")
    {
        return Err(
            "structured_output_incomplete: provider did not finish a complete JSON response".into(),
        );
    }
    let raw = choice["message"]["content"]
        .as_str()
        .ok_or("Provider response did not include content")?;
    let parsed: Value = serde_json::from_str(raw)
        .map_err(|e| format!("Provider response was not valid JSON: {e}"))?;
    // Capture limits apply after trimming, matching its canonical validator.
    // Normalize with that validator before local schema bounds to avoid rejecting
    // an otherwise valid boundary value solely for surrounding whitespace.
    let parsed = if id == PromptId::CaptureDistillation {
        crate::native::capture_distillation::validate_result(&parsed, usize::MAX, usize::MAX)?
    } else {
        parsed
    };
    let schema = schema(id, context)?;
    validate_shape(&schema, &schema, &parsed, "", true).map_err(|error| {
        if id == PromptId::WorkPreviewFinalizer {
            format!("preview_response_schema: {error}")
        } else {
            error
        }
    })?;
    let mut decoded = decode(id, parsed)?;
    if id == PromptId::WorkPreviewFinalizer {
        super::preview_references::restore(context, &mut decoded)?;
    }
    Ok(decoded)
}
// Validate the exact schema subset emitted above. Missing fields remain compatible
// with established local defaults; supplied wrong types/unknown fields never pass.
fn validate_shape(
    root: &Value,
    schema: &Value,
    value: &Value,
    path: &str,
    allow_missing: bool,
) -> Result<(), String> {
    let fail = |why: &str| format!("invalid_prompt_output: {path}: {why}");
    if let Some(reference) = schema["$ref"].as_str() {
        let pointer = reference
            .strip_prefix('#')
            .ok_or_else(|| fail("unsupported schema reference"))?;
        return validate_shape(
            root,
            root.pointer(pointer)
                .ok_or_else(|| fail("missing schema reference"))?,
            value,
            path,
            allow_missing,
        );
    }
    if let Some(options) = schema["anyOf"].as_array() {
        return if options
            .iter()
            .any(|s| validate_shape(root, s, value, path, allow_missing).is_ok())
        {
            Ok(())
        } else {
            Err(fail("value does not match any allowed schema type"))
        };
    }
    if let Some(values) = schema["enum"].as_array() {
        if !values.contains(value) {
            return Err(fail("value outside enum"));
        }
    }
    let matches = |ty: &str| match ty {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "null" => value.is_null(),
        _ => false,
    };
    let kinds = schema.get("type");
    if let Some(kind) = kinds.and_then(Value::as_str) {
        if !matches(kind) {
            return Err(fail(&format!(
                "expected {kind}, got {}",
                if value.is_array() {
                    "array"
                } else if value.is_object() {
                    "object"
                } else if value.is_null() {
                    "null"
                } else {
                    "scalar"
                }
            )));
        }
    } else if let Some(kinds) = kinds.and_then(Value::as_array) {
        if !kinds.iter().any(|k| k.as_str().is_some_and(matches)) {
            return Err(fail("incorrect JSON type"));
        }
    }
    if schema["minLength"].as_u64().is_some_and(|n| {
        value
            .as_str()
            .is_some_and(|s| s.chars().count() < n as usize)
    }) {
        return Err(fail("text below schema character minimum"));
    }
    // The emitted schemas use only this non-whitespace pattern.
    if schema["pattern"].as_str() == Some("\\S")
        && value.as_str().is_some_and(|s| s.trim().is_empty())
    {
        return Err(fail("text must contain a non-whitespace character"));
    }
    if schema["minItems"]
        .as_u64()
        .is_some_and(|n| value.as_array().is_some_and(|a| a.len() < n as usize))
    {
        return Err(fail("array below schema item minimum"));
    }
    if let Some(limit) = schema["maxLength"].as_u64() {
        if value
            .as_str()
            .is_some_and(|s| s.chars().take(limit as usize + 1).count() > limit as usize)
        {
            return Err(fail("text exceeds schema character limit"));
        }
    }
    if let Some(limit) = schema["maxItems"].as_u64() {
        if value.as_array().is_some_and(|a| a.len() > limit as usize) {
            return Err(fail("array exceeds schema item limit"));
        }
    }
    if let Some(o) = value.as_object() {
        let properties = schema["properties"].as_object();
        if !allow_missing {
            for key in schema["required"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !o.contains_key(key) {
                    return Err(fail(&format!("missing {key}")));
                }
            }
        }
        for (key, v) in o {
            let child = properties.and_then(|p| p.get(key));
            if let Some(s) = child {
                validate_shape(root, s, v, &format!("{path}.{key}"), allow_missing)?
            } else if schema["additionalProperties"] == false {
                return Err(fail(&format!("unknown field {key}")));
            }
        }
    }
    if let Some(a) = value.as_array() {
        if let Some(s) = schema.get("items") {
            for (index, v) in a.iter().enumerate() {
                validate_shape(root, s, v, &format!("{path}[{index}]"), allow_missing)?
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planner_wire_rejects_runtime_invalid_intents_before_dispatch() {
        let base = json!({"needed":false,"reason":"No search needed","queries":[],"aspects":[],"filters":{"informationTypes":[],"statuses":[]},"requery":null});
        for (key, bad) in [
            ("queries", json!(["unused query"])),
            ("requery", json!({"reason":"","queries":[]})),
            ("aspects", json!(["constraints"])),
        ] {
            let mut intent = base.clone();
            intent[key] = bad;
            let output = json!({"retrieval":intent,"preliminaryAssumptions":[]});
            assert!(
                parse_completion(
                    PromptId::WorkPreviewPlanner,
                    &json!({}),
                    &completion(output)
                )
                .is_err(),
                "invalid {key} must not be schema-valid"
            );
        }
    }
    #[test]
    fn planner_wire_query_limits_match_unicode_runtime_limits() {
        let base = json!({"needed":true,"reason":"Find evidence","queries":["\u{d55c}".repeat(500)],"aspects":["content"],"filters":{"informationTypes":[],"statuses":[]},"requery":null});
        let parse = |intent: Value| {
            parse_completion(
                PromptId::WorkPreviewPlanner,
                &json!({}),
                &completion(json!({"retrieval":intent,"preliminaryAssumptions":[]})),
            )
        };
        assert!(parse(base.clone()).is_ok());
        for bad in [
            json!([]),
            json!([" "]),
            json!(["valid", ""]),
            json!(["\u{d55c}".repeat(501)]),
            json!(["1", "2", "3", "4", "5"]),
        ] {
            let mut intent = base.clone();
            intent["queries"] = bad;
            assert!(parse(intent).is_err());
        }
        for bad in [
            json!({"reason":"retry","queries":[]}),
            json!({"reason":" ","queries":["valid"]}),
            json!({"reason":"retry","queries":["1","2","3"]}),
        ] {
            let mut intent = base.clone();
            intent["requery"] = bad;
            assert!(parse(intent).is_err());
        }
    }
    fn completion(value: Value) -> Value {
        json!({"choices":[{"finish_reason":"stop","message":{"content":value.to_string()}}]})
    }
    fn sample(root: &Value, s: &Value) -> Value {
        if let Some(r) = s["$ref"].as_str() {
            return sample(root, root.pointer(&r[1..]).unwrap());
        }
        if let Some(a) = s["anyOf"].as_array() {
            return sample(
                root,
                a.iter().find(|v| v["type"] == "null").unwrap_or(&a[0]),
            );
        }
        if let Some(a) = s["enum"].as_array() {
            return a[0].clone();
        }
        match s["type"]
            .as_str()
            .or_else(|| s["type"][0].as_str())
            .unwrap_or("")
        {
            "object" => Value::Object(
                s["properties"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), sample(root, v)))
                    .collect(),
            ),
            "array" => json!([]),
            "integer" | "number" => json!(1),
            "boolean" => json!(false),
            "null" => Value::Null,
            _ => json!("text"),
        }
    }
    #[test]
    fn every_prompt_has_a_strict_contract_matching_its_validator() {
        let context = json!({"events":{"events":[{"id":"e1"},{"id":"e2"}]},"semanticTargetRevisions":{"node":2}});
        for id in super::super::ALL_PROMPT_IDS {
            match schema(id, &context) {
                Ok(s) => {
                    assert_eq!(s["type"], "object", "{id}");
                    let body = request_body(id, &context, "fixture", json!("prompt")).unwrap();
                    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
                    let mut value = sample(&s, &s);
                    if id == PromptId::CaptureDistillation {
                        value["grounding"] = json!([{"field":"title","sourceRefs":["text:0-4"]},{"field":"content","sourceRefs":["text:0-4"]},{"field":"context","sourceRefs":["text:0-4"]}]);
                    }
                    validate_shape(&s, &s, &value, "", false).unwrap();
                    let parsed = parse_completion(id, &context, &completion(value)).unwrap();
                    super::super::validate_prompt_output(id, &parsed).unwrap();
                }
                Err(e) => panic!("registered prompt {id} has no schema: {e}"),
            }
        }
    }
    #[test]
    fn capture_schema_limits_match_runtime_without_unsupported_wire_keywords() {
        use crate::native::capture_distillation::{
            CONTENT_LIMIT, CONTEXT_LIMIT, REQUEST_COUNT_LIMIT, REQUEST_LIMIT, TITLE_LIMIT,
        };
        let local = schema(PromptId::CaptureDistillation, &Value::Null).unwrap();
        for (field, limit) in [
            ("title", TITLE_LIMIT),
            ("content", CONTENT_LIMIT),
            ("context", CONTEXT_LIMIT),
        ] {
            assert_eq!(local["properties"][field]["maxLength"], limit);
        }
        assert_eq!(
            local["properties"]["explicitRequests"]["maxItems"],
            REQUEST_COUNT_LIMIT
        );
        assert_eq!(
            local["properties"]["explicitRequests"]["items"]["maxLength"],
            REQUEST_LIMIT
        );
        let request = request_body(
            PromptId::CaptureDistillation,
            &Value::Null,
            "fixture",
            json!("prompt"),
        )
        .unwrap();
        let wire = &request["response_format"]["json_schema"]["schema"];
        assert!(!wire.to_string().contains("maxLength"));
        assert!(!wire.to_string().contains("maxItems"));
        assert!(wire["properties"]["content"]["description"]
            .as_str()
            .unwrap()
            .contains("20000"));
        assert!(wire["properties"]["context"]["description"]
            .as_str()
            .unwrap()
            .contains("10000"));
        assert_eq!(request["response_format"]["json_schema"]["strict"], true);
        let prompt =
            super::super::build_prompt(PromptId::CaptureDistillation, &json!({"context":{}}))
                .unwrap()
                .content;
        assert!(prompt.contains("20000 Unicode scalar values"));
        assert!(prompt.contains("10000 Unicode scalar values"));
        assert!(prompt.contains("exclusive end"));
        assert!(prompt.contains("maximums, not target lengths"));
        let valid = json!({"schemaVersion":1,"title":"test","content":"","context":"","explicitRequests":[],"grounding":[{"field":"title","sourceRefs":["text:0-4"]}]});
        let mut boundary = valid.clone();
        boundary["content"] = json!(format!("  {}\n", "한😀".repeat(CONTENT_LIMIT / 2)));
        boundary["grounding"]
            .as_array_mut()
            .unwrap()
            .push(json!({"field":"content","sourceRefs":["text:0-4"]}));
        let parsed = parse_completion(
            PromptId::CaptureDistillation,
            &Value::Null,
            &completion(boundary),
        )
        .unwrap();
        assert_eq!(
            parsed["content"].as_str().unwrap().chars().count(),
            CONTENT_LIMIT
        );
        let mut too_long = valid.clone();
        too_long["content"] = json!("가".repeat(CONTENT_LIMIT + 1));
        assert!(parse_completion(
            PromptId::CaptureDistillation,
            &Value::Null,
            &completion(too_long)
        )
        .unwrap_err()
        .contains("content is too long"));
        assert!(parse_completion(
            PromptId::CaptureDistillation,
            &Value::Null,
            &json!({"choices":[{"finish_reason":"length","message":{"content":valid.to_string()}}]})
        )
        .unwrap_err()
        .contains("structured_output_incomplete"));
    }

    #[test]
    fn scope_is_a_string_and_arrays_are_never_coerced() {
        let s = schema(PromptId::WorkPreviewFinalizer, &Value::Null).unwrap();
        assert_eq!(
            s["properties"]["preview"]["properties"]["scope"]["type"],
            "string"
        );
        let mut value = sample(&s, &s);
        value["preview"]["scope"] = json!(["one", "two"]);
        let error = parse_completion(
            PromptId::WorkPreviewFinalizer,
            &Value::Null,
            &completion(value),
        )
        .unwrap_err();
        assert!(
            error.contains("preview.scope")
                && error.contains("expected string")
                && error.contains("array"),
            "{error}"
        );
    }
    #[test]
    fn dynamic_maps_preserve_known_identity_and_sparse_revisions() {
        let context = json!({"semanticTargetRevisions":{"node-a":2,"node-b":4}});
        let s = schema(PromptId::RunReportDistillation, &context).unwrap();
        let refs = &s["$defs"]["NodeChange"]["properties"]["expectedTargetRevisions"];
        assert!(refs["properties"].get("node-a").is_some());
        let mut value = sample(&s, &s);
        let node = sample(&s, &s["$defs"]["NodeChange"]);
        value["nodeChanges"] = json!([node]);
        value["nodeChanges"][0]["expectedTargetRevisions"] = json!({"node-a":2,"node-b":null});
        let parsed = parse_completion(
            PromptId::RunReportDistillation,
            &context,
            &completion(value.clone()),
        )
        .unwrap();
        assert_eq!(
            parsed["nodeChanges"][0]["expectedTargetRevisions"],
            json!({"node-a":2})
        );
        value["nodeChanges"][0]["expectedTargetRevisions"] = json!({"invented":9});
        assert!(parse_completion(
            PromptId::RunReportDistillation,
            &context,
            &completion(value)
        )
        .is_err());
    }
    #[test]
    fn omission_compatibility_and_refusal_are_explicit() {
        let raw = completion(json!({"preview":{"description":"Review","goal":"Test"}}));
        let mut value =
            parse_completion(PromptId::WorkPreviewFinalizer, &Value::Null, &raw).unwrap();
        super::super::normalize_prompt_output(PromptId::WorkPreviewFinalizer, &mut value);
        super::super::validate_prompt_output(PromptId::WorkPreviewFinalizer, &value).unwrap();
        for bad in [
            json!({"choices":[{"message":{"refusal":"No"}}]}),
            json!({"choices":[{"finish_reason":"length","message":{"content":"{}"}}]}),
        ] {
            assert!(parse_completion(PromptId::WorkPreviewFinalizer, &Value::Null, &bad).is_err());
        }
    }
    #[test]
    fn generic_wire_roundtrips_nested_json_without_inventing_nulls() {
        let context = Value::Null;
        let raw = completion(
            json!({"proposal":{"_entries":[{"key":"scope","value":["a","b"]},{"key":"nested","value":{"_entries":[{"key":"_entries","value":"literal key"},{"key":"flag","value":false}]}},{"key":"null","value":null}]}}),
        );
        let parsed = parse_completion(PromptId::UserGeneration, &context, &raw).unwrap();
        assert_eq!(
            parsed,
            json!({"proposal":{"scope":["a","b"],"nested":{"_entries":"literal key","flag":false},"null":null}})
        );
        for entries in [
            json!([{"key":"x"}]),
            json!([{"value":1}]),
            json!([null]),
            json!([{"key":"x","value":1},{"key":"x","value":2}]),
        ] {
            let bad = completion(json!({"proposal":{"_entries":entries}}));
            assert!(parse_completion(PromptId::UserGeneration, &context, &bad).is_err());
        }
    }
    #[test]
    fn sparse_legacy_preview_patch_preserves_unchanged_fields() {
        let context = Value::Null;
        let s = schema(PromptId::RefinementPreview, &context).unwrap();
        let mut proposal = sample(&s, &s["properties"]["proposals"]["items"]["anyOf"][0]);
        proposal["payload"]["patch"]["scope"] = json!("Only API");
        let parsed = parse_completion(
            PromptId::RefinementPreview,
            &context,
            &completion(json!({"proposals":[proposal]})),
        )
        .unwrap();
        assert_eq!(
            parsed["proposals"][0]["payload"]["patch"],
            json!({"scope":"Only API"})
        );
    }
    #[test]
    fn structured_contract_local_cost_receipt() {
        for id in [
            PromptId::WorkPreviewFinalizer,
            PromptId::KnowledgeDraft,
            PromptId::RunReportDistillation,
            PromptId::TaskJourney,
        ] {
            let context =
                json!({"semanticTargetRevisions":{"node":2},"events":{"events":[{"id":"event"}]}});
            let s = schema(id, &context).unwrap();
            let payload = completion(sample(&s, &s));
            let mut timings = Vec::new();
            let mut request_bytes = 0;
            for _ in 0..100 {
                let start = std::time::Instant::now();
                let body = request_body(id, &context, "fixture", json!("prompt")).unwrap();
                request_bytes = serde_json::to_vec(&body).unwrap().len();
                parse_completion(id, &context, &payload).unwrap();
                timings.push(start.elapsed().as_micros());
            }
            timings.sort_unstable();
            eprintln!("schema local receipt {id}: 100 request-serialize+response-parse iterations, p50={}us p95={}us request_bytes={request_bytes}; no network/model calls",timings[49],timings[94]);
        }
    }
}

#[cfg(test)]
pub(crate) mod test_server {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    pub(crate) async fn serve(
        result: Value,
        status: u16,
    ) -> (String, tokio::task::JoinHandle<Value>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 4096];
            let end = loop {
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let length = String::from_utf8_lossy(&bytes[..end])
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break end;
                    }
                }
            };
            let sent: Value = serde_json::from_slice(&bytes[end + 4..]).unwrap();
            let payload=json!({"choices":[{"finish_reason":"stop","message":{"content":result.to_string()}}]}).to_string();
            socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",payload.len()).as_bytes()).await.unwrap();
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(50), listener.accept())
                    .await
                    .is_err(),
                "schema failure must not create another model call"
            );
            sent
        });
        (base, task)
    }
}
