use crate::workflow_foundation::{build_prompt, validate_prompt_output, PromptId};
use crate::{native, NativeApplication, NativeOperation, NativeResponse};
use serde_json::{json, Value};

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
        let client = native::settings::provider_client(&application.settings_path())?;
        if operation.name == "problem.enrich" {
            return enrich(&client, &base_url, &model, &api_key, &operation.input).await;
        }
        let response = client
            .get(format!("{}/models", base_url.trim_end_matches('/')))
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

async fn enrich(
    client: &reqwest::Client,
    base_url: &str,
    model: &str,
    api_key: &str,
    input: &Value,
) -> Result<Value, String> {
    let statement = input
        .get("statement")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or("statement is required")?;
    let citations = input
        .get("citations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(8)
        .collect::<Vec<_>>()
        .join(", ");
    let context = json!({"statement":statement,"citations":citations});
    let prompt = build_prompt(PromptId::ProblemEnrichment, &json!({"context":context}))
        .map_err(|error| error.to_string())?;
    let response = client
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(
            &crate::workflow_foundation::structured_output::request_body(
                PromptId::ProblemEnrichment,
                &context,
                model,
                json!(prompt.content),
            )?,
        )
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(crate::workflow_foundation::structured_output::http_error(
            response.status(),
        ));
    }
    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())?;
    let mut result = crate::workflow_foundation::structured_output::parse_completion(
        PromptId::ProblemEnrichment,
        &context,
        &payload,
    )?;
    validate_prompt_output(PromptId::ProblemEnrichment, &result)
        .map_err(|error| error.to_string())?;
    result["_prompt"] = json!({"id":prompt.id.as_str(),"version":prompt.version});
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn structured_problem_http_request_and_response_use_the_same_schema() {
        let result = json!({"normalized_problem":"Manual export","pain":"Slow","non_goals":[],"categories":[],"importance_rationale":"Recorded delay"});
        let (base, server) =
            crate::workflow_foundation::structured_output::test_server::serve(result.clone(), 200)
                .await;
        let parsed = enrich(
            &reqwest::Client::new(),
            &base,
            "fixture",
            "fixture-key",
            &json!({"statement":"Export"}),
        )
        .await
        .unwrap();
        assert_eq!(parsed["normalized_problem"], result["normalized_problem"]);
        let sent = server.await.unwrap();
        assert_eq!(
            sent["response_format"]["json_schema"]["name"],
            "problem_enrichment"
        );
        assert_eq!(sent["response_format"]["json_schema"]["strict"], true);
    }
}
