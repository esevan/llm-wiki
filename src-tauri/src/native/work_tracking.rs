use crate::application::work_tracking_service::WorkTrackingApplicationService;
use crate::domain::work_tracking_state::AppError;
use crate::native::{NativeOperation, NativeResponse};
use serde_json::{json, Value};

const NATIVE_CONNECTION: &str = "native-in-app-chat";

pub fn execute(
    service: &WorkTrackingApplicationService,
    operation: NativeOperation,
) -> NativeResponse {
    match dispatch(service, &operation.name, &operation.input) {
        Ok((status, body)) => NativeResponse { status, body },
        Err(error) => NativeResponse {
            status: status(&error),
            body: json!({"error":error}),
        },
    }
}

fn dispatch(
    service: &WorkTrackingApplicationService,
    name: &str,
    input: &Value,
) -> Result<(u16, Value), AppError> {
    match name {
        "work_tracking.connection.create" => {
            let scopes = input
                .get("scopes")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let topics = input
                .get("topicIds")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Ok((
                201,
                service.create_connection(
                    input
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("Local Chat"),
                    &scopes,
                    &topics,
                    input
                        .get("checkpointPolicy")
                        .and_then(Value::as_str)
                        .unwrap_or("confirm_each"),
                )?,
            ))
        }
        "work_tracking.connection.list" => Ok((200, service.list_connections()?)),
        "work_tracking.connection.revoke" => {
            service.revoke_connection(required(input, "connectionId")?)?;
            Ok((204, Value::Null))
        }
        "work_tracking.open" => Ok((200, service.open(NATIVE_CONNECTION, input)?)),
        "work_tracking.selection" => Ok((200, service.select_work(input)?)),
        "work_tracking.topic.membership" => Ok((200, service.set_topic_membership(input)?)),
        "work_tracking.open.review" => Ok((
            200,
            service.finish_open(
                NATIVE_CONNECTION,
                &input["proposal"],
                required(input, "reviewState")?,
                required(input, "decision")?,
            )?,
        )),
        "work_tracking.append" => Ok((202, service.append(NATIVE_CONNECTION, input)?)),
        "work_tracking.session" => Ok((
            200,
            service.session(NATIVE_CONNECTION, required(input, "sessionId")?)?,
        )),
        "work_tracking.decide" => Ok((
            200,
            service.decide(NATIVE_CONNECTION, input, "in_app_chat")?,
        )),
        "work_tracking.advance" => Err(AppError::new(
            "approval_required",
            "Review the exact workflow proposal before advancing",
        )),
        "work_tracking.advance.preview" => {
            let state = service.begin_advance(NATIVE_CONNECTION, input)?;
            Ok((
                200,
                json!({"target":service.review_target(NATIVE_CONNECTION,&state)?,"reviewState":state,"proposal":input}),
            ))
        }
        "work_tracking.advance.review" => Ok((
            200,
            service.finish_advance(
                NATIVE_CONNECTION,
                required(input, "reviewState")?,
                &input["proposal"],
                required(input, "decision")?,
            )?,
        )),
        "work_tracking.current" => Ok((
            200,
            service.current_workbench(
                NATIVE_CONNECTION,
                input.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize,
            )?,
        )),
        "work_tracking.overview" => Ok((
            200,
            service.overview(
                NATIVE_CONNECTION,
                input.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize,
                input.get("cursor").and_then(Value::as_str),
                input.get("snapshotRevision").and_then(Value::as_i64),
            )?,
        )),
        "work_tracking.vault.lexical" => Ok((
            200,
            service.lexical_search(
                NATIVE_CONNECTION,
                input
                    .get("scope")
                    .and_then(Value::as_str)
                    .unwrap_or("workbench"),
                input.get("targetId").and_then(Value::as_str).unwrap_or(""),
                required(input, "query")?,
                input.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize,
            )?,
        )),
        "work_tracking.vault.semantic" => Ok((
            200,
            service.semantic_search(
                NATIVE_CONNECTION,
                input
                    .get("scope")
                    .and_then(Value::as_str)
                    .unwrap_or("workbench"),
                input.get("targetId").and_then(Value::as_str).unwrap_or(""),
                required(input, "query")?,
                input.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize,
            )?,
        )),
        "work_tracking.vault.evidence" => Ok((
            200,
            service.evidence_read(
                NATIVE_CONNECTION,
                input
                    .get("items")
                    .and_then(Value::as_array)
                    .ok_or_else(|| AppError::new("invalid_input", "items is required"))?,
            )?,
        )),
        "work_tracking.knowledge.draft.save" => {
            Ok((201, service.save_knowledge_draft(NATIVE_CONNECTION, input)?))
        }
        "work_tracking.knowledge.publish" => {
            Ok((200, service.publish_knowledge(NATIVE_CONNECTION, input)?))
        }
        "work_tracking.knowledge.draft.review" => Ok((
            201,
            service.finish_knowledge_draft(
                NATIVE_CONNECTION,
                &input["proposal"],
                required(input, "reviewState")?,
                required(input, "decision")?,
            )?,
        )),
        "work_tracking.knowledge.publish.preview" => {
            Ok((200, service.begin_publish(NATIVE_CONNECTION, input)?))
        }
        "work_tracking.knowledge.withdraw.preview" => {
            Ok((200, service.begin_withdrawal(NATIVE_CONNECTION, input)?))
        }
        "work_tracking.knowledge.withdraw.review" => Ok((
            200,
            service.finish_withdrawal(
                NATIVE_CONNECTION,
                &input["proposal"],
                required(input, "reviewState")?,
                required(input, "decision")?,
            )?,
        )),
        "work_tracking.knowledge.publish.review" => Ok((
            200,
            service.finish_publish(
                NATIVE_CONNECTION,
                &input["proposal"],
                required(input, "reviewState")?,
                required(input, "decision")?,
            )?,
        )),
        "work_tracking.knowledge.defer" => Ok((
            200,
            service.defer_publication(
                NATIVE_CONNECTION,
                required(input, "sessionId")?,
                input
                    .get("completionRevision")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| {
                        AppError::new("invalid_input", "completionRevision is required")
                    })?,
            )?,
        )),
        "work_tracking.project" => Ok((
            200,
            json!({"processed":service.drain(input.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize)?}),
        )),
        _ => Err(AppError::new(
            "invalid_input",
            "Unsupported work-tracking operation",
        )),
    }
}

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::new("invalid_input", format!("{key} is required")))
}

fn status(error: &AppError) -> u16 {
    match error.code.as_str() {
        "invalid_input" | "content_too_large" => 400,
        "approval_required" | "elicitation_required" => 403,
        "not_found_or_not_visible" => 404,
        "head_conflict"
        | "idempotency_conflict"
        | "session_closed"
        | "publish_conflict"
        | "evidence_revision_changed"
        | "workflow_precondition"
        | "challenge_expired"
        | "challenge_replayed"
        | "snapshot_stale" => 409,
        "semantic_index_not_ready" => 503,
        _ => 500,
    }
}
