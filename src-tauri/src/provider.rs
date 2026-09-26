use crate::{native, NativeApplication, NativeOperation, NativeResponse};
use serde_json::{json, Value};
use std::time::Duration;
use crate::workflow_foundation::{build_prompt, validate_prompt_output, PromptId};

#[tauri::command]
pub(crate) async fn provider_request(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    if !matches!(operation.name.as_str(), "provider.test" | "problem.enrich") {
        return Ok(NativeResponse {
            status: 400,
            body: json!({"detail":"Unsupported provider command"}),
        });
    }
    let outcome = async {
        let (base_url, model, api_key) = native::settings::provider_credentials_for(
            &application.settings_path(),
            "problem_enrichment",
        )?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| error.to_string())?;
        if operation.name == "problem.enrich" {
            let statement = operation.input.get("statement").and_then(Value::as_str).filter(|value| !value.trim().is_empty()).ok_or("statement is required")?;
            let citations = operation.input.get("citations").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).take(8).collect::<Vec<_>>().join(", ");
            let prompt = build_prompt(
                PromptId::ProblemEnrichment,
                &json!({"context":{"statement":statement,"citations":citations}}),
            ).map_err(|error| error.to_string())?;
            let response = client.post(format!("{}/chat/completions", base_url.trim_end_matches('/')))
                .bearer_auth(api_key).json(&json!({"model":model,"messages":[{"role":"user","content":prompt.content}],"stream":false})).send().await.map_err(|error| error.to_string())?;
            if !response.status().is_success() { return Err(format!("Problem enrichment failed ({})", response.status())); }
            let payload = response.json::<Value>().await.map_err(|error| error.to_string())?;
            let raw = payload.pointer("/choices/0/message/content").and_then(Value::as_str).ok_or("Problem enrichment response did not include content")?;
            let mut result = serde_json::from_str::<Value>(raw).map_err(|error| error.to_string())?;
            validate_prompt_output(PromptId::ProblemEnrichment, &result)
                .map_err(|error| error.to_string())?;
            result["_prompt"] = json!({"id":prompt.id.as_str(),"version":prompt.version});
            return Ok(result);
        }
        let response = client.get(format!("{}/models", base_url.trim_end_matches('/')))
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Provider health check failed ({})",
                response.status()
            ));
        }
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| error.to_string())?;
        let models = payload
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| item.get("id").and_then(Value::as_str))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Ok::<_, String>(json!({"models":models,"configured_model":model}))
    }
    .await;
    Ok(match outcome {
        Ok(body) => NativeResponse { status: 200, body },
        Err(error) => NativeResponse {
            status: 502,
            body: json!({"detail":error}),
        },
    })
}
