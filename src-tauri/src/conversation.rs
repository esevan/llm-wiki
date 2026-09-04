use crate::{native, NativeApplication, NativeResponse};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Default)]
pub(crate) struct RequestRegistry {
    active: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum StreamEvent {
    Chunk { data: Vec<u8> },
    Complete,
    Cancelled,
    Error { message: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationInput {
    request_id: String,
    entity_type: String,
    entity_id: String,
    message: String,
    mode: String,
    locale: String,
}

impl RequestRegistry {
    async fn register(&self, request_id: &str) -> Result<CancellationToken, String> {
        if request_id.is_empty() || request_id.len() > 128 {
            return Err("Invalid request identifier".into());
        }
        let mut active = self.active.lock().await;
        if active.contains_key(request_id) {
            return Err("Request identifier is already active".into());
        }
        let token = CancellationToken::new();
        active.insert(request_id.to_owned(), token.clone());
        Ok(token)
    }

    async fn finish(&self, request_id: &str) {
        self.active.lock().await.remove(request_id);
    }

    async fn cancel(&self, request_id: &str) -> bool {
        if let Some(token) = self.active.lock().await.remove(request_id) {
            token.cancel();
            true
        } else {
            false
        }
    }
}

#[tauri::command]
pub(crate) async fn conversation_stream(
    application: tauri::State<'_, NativeApplication>,
    registry: tauri::State<'_, RequestRegistry>,
    input: ConversationInput,
    on_event: Channel<StreamEvent>,
) -> Result<NativeResponse, String> {
    let ConversationInput {
        request_id,
        entity_type,
        entity_id,
        message,
        mode,
        locale,
    } = input;
    if message.trim().is_empty() {
        return Err("message is required".into());
    }
    let request = native::conversation_context::build(
        &application.db_path(),
        &entity_type,
        &entity_id,
        &mode,
        &locale,
        &message,
    )?;
    let (base_url, model, api_key) = native::settings::provider_credentials_for(
        &application.settings_path(),
        request.model_task,
    )?;
    if model.trim().is_empty() {
        return Err("Provider model is required".into());
    }
    let token = registry.register(&request_id).await?;
    let registry = registry.inner().clone();
    let db_path = application.db_path();
    tauri::async_runtime::spawn(async move {
        let task = async {
            let response = reqwest::Client::new()
                .post(format!(
                    "{}/chat/completions",
                    base_url.trim_end_matches('/')
                ))
                .bearer_auth(api_key)
                .json(&json!({
                    "model": model,
                    "messages": request.messages,
                    "stream": true,
                }))
                .send()
                .await
                .map_err(|error| error.to_string())?;
            if !response.status().is_success() {
                return Err(format!("Provider request failed ({})", response.status()));
            }
            let mut source = response.bytes_stream();
            let mut buffer = String::new();
            let mut output = String::new();
            loop {
                let chunk = tokio::select! {
                    _ = token.cancelled() => {
                        let _ = on_event.send(StreamEvent::Cancelled);
                        return Ok(());
                    }
                    chunk = source.next() => chunk,
                };
                let Some(chunk) = chunk else {
                    break;
                };
                let chunk = chunk.map_err(|error| error.to_string())?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                while let Some(end) = buffer.find('\n') {
                    let line = buffer[..end].trim_end_matches('\r').to_owned();
                    buffer.drain(..=end);
                    let Some(data) = line.strip_prefix("data: ") else {
                        continue;
                    };
                    if data == "[DONE]" {
                        break;
                    }
                    let value: serde_json::Value =
                        serde_json::from_str(data).map_err(|error| error.to_string())?;
                    let Some(text) = value
                        .pointer("/choices/0/delta/content")
                        .and_then(serde_json::Value::as_str)
                    else {
                        continue;
                    };
                    let remaining = 1_200usize.saturating_sub(output.chars().count());
                    if remaining == 0 {
                        break;
                    }
                    let compact = text.chars().take(remaining).collect::<String>();
                    output.push_str(&compact);
                    let data = format!("data: {}\n\n", compact.replace('\n', " ")).into_bytes();
                    if on_event.send(StreamEvent::Chunk { data }).is_err() {
                        token.cancel();
                        return Ok(());
                    }
                }
                if token.is_cancelled() {
                    let _ = on_event.send(StreamEvent::Cancelled);
                    return Ok(());
                }
            }
            native::workflow::record_ai_run(&db_path, &entity_type, &entity_id, &message, &output)?;
            let _ = on_event.send(StreamEvent::Chunk {
                data: b"event: done\ndata: done\n\n".to_vec(),
            });
            let _ = on_event.send(StreamEvent::Complete);
            Ok::<(), String>(())
        }
        .await;
        if let Err(message) = task {
            let _ = on_event.send(StreamEvent::Error { message });
        }
        registry.finish(&request_id).await;
    });
    Ok(NativeResponse {
        status: 200,
        body: serde_json::Value::Null,
    })
}

#[tauri::command]
pub(crate) async fn cancel_conversation(
    registry: tauri::State<'_, RequestRegistry>,
    request_id: String,
) -> Result<bool, String> {
    Ok(registry.cancel(&request_id).await)
}
