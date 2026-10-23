//! Failure metadata only: never persist a rejected response or source text.
use super::*;

#[derive(Default)]
pub(crate) struct ResponseDiagnostics {
    candidate: Option<Value>,
    candidate_state: &'static str,
    finish_reason: Option<&'static str>,
}

impl ResponseDiagnostics {
    pub(crate) fn from_payload(payload: &Value) -> Self {
        let choice = &payload["choices"][0];
        let finish_reason = choice["finish_reason"].as_str().map(|reason| match reason {
            "stop" => "stop",
            "length" => "length",
            "content_filter" => "content_filter",
            "tool_calls" => "tool_calls",
            "function_call" => "function_call",
            _ => "other",
        });
        let raw = choice["message"]["content"].as_str();
        let candidate = raw.and_then(|raw| serde_json::from_str::<Value>(raw).ok());
        let candidate_state = match &candidate {
            Some(Value::Object(object)) if object.is_empty() => "empty_object",
            Some(Value::Object(_)) => "object",
            Some(Value::Array(_)) => "array",
            Some(Value::Null) => "null",
            Some(_) => "scalar",
            None if raw.is_some() => "invalid_json",
            None => "content_absent",
        };
        Self {
            candidate,
            candidate_state,
            finish_reason,
        }
    }

    fn metadata(&self, input: &Value, stage: &'static str, error: &str) -> Value {
        let text_length = input["text"]
            .as_str()
            .map_or(0, |text| text.chars().count());
        let image_count = input["images"].as_array().map_or(0, Vec::len);
        let issue = self.candidate.as_ref().and_then(|candidate| {
            super::validate_result_detailed(candidate, text_length, image_count).err()
        });
        let failure = if error.contains("structured_output_refused") {
            json!({"field":"response","rule":"refused"})
        } else if self.finish_reason.is_some_and(|reason| reason != "stop") {
            json!({"field":"response","rule":"incomplete"})
        } else if let Some(issue) = issue {
            json!({"field":issue.field,"rule":issue.rule,"observed":issue.observed,"limit":issue.limit})
        } else {
            // The reason is a fixed category; errors from transports or user-supplied
            // property names must never become unbounded diagnostic strings.
            let rule = if error.contains("structured_output_refused") {
                "refused"
            } else if error.contains("not valid JSON") {
                "invalid_json"
            } else if error.contains("invalid_prompt_output") {
                "schema_mismatch"
            } else if error.contains("structured_output_incomplete") {
                "incomplete"
            } else {
                "unavailable_or_not_applied"
            };
            json!({"field":"response","rule":rule})
        };
        let mut lengths = serde_json::Map::new();
        if let Some(candidate) = &self.candidate {
            for field in ["title", "content", "context"] {
                if let Some(text) = candidate[field].as_str() {
                    lengths.insert(field.into(), json!(text.trim().chars().count()));
                }
            }
            if let Some(requests) = candidate["explicitRequests"].as_array() {
                lengths.insert("explicitRequestCount".into(), json!(requests.len()));
                // Only a fixed-size numeric sample, even for an invalid huge array.
                lengths.insert(
                    "explicitRequests".into(),
                    json!(requests
                        .iter()
                        .take(REQUEST_COUNT_LIMIT)
                        .map(|value| value.as_str().map(|text| text.trim().chars().count()))
                        .collect::<Vec<_>>()),
                );
            }
        }
        json!({
            "schemaVersion":1,"stage":stage,"failure":failure,
            "source":{"textCharacterCount":text_length,"imageCount":image_count},
            "outputLengths":lengths,"finishReason":self.finish_reason,
            "candidate":{"state":if self.candidate_state.is_empty() { "response_absent" } else { self.candidate_state },
                "received":self.candidate.is_some(),"saved":false}
        })
    }
}

pub(crate) fn record_failure(
    connection: &rusqlite::Connection,
    job_id: &str,
    generation: &str,
    input: &Value,
    response: &ResponseDiagnostics,
    stage: &'static str,
    error: &str,
) -> Result<(), String> {
    let metadata = json!({"failureDiagnostics":response.metadata(input, stage, error)});
    connection.execute(
        "UPDATE ai_jobs_v2 SET result_json=? WHERE id=? AND task_kind='capture_distillation' AND status='running' AND worker_id=?",
        params![metadata.to_string(), job_id, generation],
    ).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_response_metadata_handles_truncation_invalid_json_and_untrusted_strings() {
        let input = json!({"text":"가😀","images":[]});
        let truncated = ResponseDiagnostics::from_payload(
            &json!({"choices":[{"finish_reason":"length","message":{"content":"{}"}}]}),
        );
        let metadata = truncated.metadata(&input, "response", "structured_output_incomplete");
        assert_eq!(metadata["finishReason"], "length");
        assert_eq!(metadata["failure"]["rule"], "incomplete");
        assert_eq!(metadata["candidate"]["state"], "empty_object");
        let invalid = ResponseDiagnostics::from_payload(
            &json!({"choices":[{"finish_reason":"stop","message":{"content":"private-incomplete-transcript{"}}]}),
        );
        let metadata = invalid.metadata(&input, "response", "Provider response was not valid JSON");
        assert_eq!(metadata["candidate"]["state"], "invalid_json");
        assert_eq!(metadata["failure"]["rule"], "invalid_json");
        assert!(!metadata
            .to_string()
            .contains("private-incomplete-transcript"));
        let malicious = ResponseDiagnostics::from_payload(
            &json!({"choices":[{"finish_reason":"private-finish-reason","message":{"content":"{\"private-property\":\"private-value\"}"}}]}),
        );
        let metadata = malicious.metadata(&input, "response", "private-error");
        assert_eq!(metadata["finishReason"], "other");
        assert!(!metadata.to_string().contains("private-"));
        for (raw, state) in [
            ("[]", "array"),
            ("null", "null"),
            ("1", "scalar"),
            ("", "invalid_json"),
        ] {
            let response = ResponseDiagnostics::from_payload(
                &json!({"choices":[{"finish_reason":"stop","message":{"content":raw}}]}),
            );
            assert_eq!(
                response.metadata(&input, "response", "invalid_prompt_output")["candidate"]
                    ["state"],
                state
            );
        }
    }

    #[test]
    fn capture_diagnostics_cannot_overwrite_another_worker_or_finished_result() {
        let (_root, db, created) = super::super::tests::saved_capture();
        let connection = crate::native::database::open(&db).unwrap();
        let job = created["distillation"]["jobId"].as_str().unwrap();
        connection
            .execute(
                "UPDATE ai_jobs_v2 SET status='running',worker_id='new-worker' WHERE id=?",
                [job],
            )
            .unwrap();
        record_failure(
            &connection,
            job,
            "old-worker",
            &Value::Null,
            &ResponseDiagnostics::default(),
            "response",
            "error",
        )
        .unwrap();
        let raw = || {
            connection
                .query_row(
                    "SELECT result_json FROM ai_jobs_v2 WHERE id=?",
                    [job],
                    |row| row.get::<_, String>(0),
                )
                .unwrap()
        };
        assert_eq!(raw(), "{}");
        record_failure(
            &connection,
            job,
            "new-worker",
            &Value::Null,
            &ResponseDiagnostics::default(),
            "response",
            "error",
        )
        .unwrap();
        assert!(raw().contains("response_absent"));
        connection
            .execute(
                "UPDATE ai_jobs_v2 SET status='completed',result_json='{}' WHERE id=?",
                [job],
            )
            .unwrap();
        record_failure(
            &connection,
            job,
            "new-worker",
            &Value::Null,
            &ResponseDiagnostics::default(),
            "response",
            "error",
        )
        .unwrap();
        assert_eq!(raw(), "{}");
    }
}
