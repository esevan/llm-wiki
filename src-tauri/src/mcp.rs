use crate::application::work_tracking_service::WorkTrackingApplicationService;
use crate::domain::work_tracking_state::AppError;
use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        tool::{InputResponses as ToolInputResponses, RequestState},
        wrapper::Parameters,
    },
    model::{
        CacheScope, CallToolResponse, CallToolResult, ElicitRequest, ElicitRequestParams,
        Implementation, InputRequest, InputRequiredResult, ListResourceTemplatesResult,
        ListResourcesResult, PaginatedRequestParams, ProtocolVersion, ReadResourceRequestParams,
        ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ResourceTemplate,
        ServerCapabilities, ServerInfo,
    },
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router,
    transport::IntoTransport,
    Json, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const WORKBENCH_OVERVIEW_URI: &str = "llm-wiki://workbench/overview";
const WORKBENCH_OVERVIEW_TEMPLATE: &str =
    "llm-wiki://workbench/overview{?snapshotRevision,cursor,limit}";

fn overview_resource_page(
    uri: &str,
) -> Result<Option<(usize, Option<&str>, Option<i64>)>, AppError> {
    let Some(query) = uri.strip_prefix(WORKBENCH_OVERVIEW_URI) else {
        return Ok(None);
    };
    if query.is_empty() {
        return Ok(Some((50, None, None)));
    }
    let Some(query) = query.strip_prefix('?') else {
        return Ok(None);
    };
    let mut limit = 50usize;
    let mut cursor = None;
    let mut snapshot_revision = None;
    let mut has_limit = false;
    for parameter in query.split('&') {
        let Some((name, value)) = parameter.split_once('=') else {
            return Err(AppError::new(
                "invalid_input",
                "Overview URI parameters are invalid",
            ));
        };
        match name {
            "limit" if !has_limit => {
                limit = value.parse().map_err(|_| {
                    AppError::new("invalid_input", "Overview limit must be a positive integer")
                })?;
                if limit == 0 {
                    return Err(AppError::new(
                        "invalid_input",
                        "Overview limit must be a positive integer",
                    ));
                }
                has_limit = true;
            }
            "cursor" if cursor.is_none() && !value.is_empty() => cursor = Some(value),
            "snapshotRevision" if snapshot_revision.is_none() => {
                snapshot_revision = Some(value.parse().map_err(|_| {
                    AppError::new(
                        "invalid_input",
                        "Overview snapshotRevision must be an integer",
                    )
                })?);
            }
            _ => {
                return Err(AppError::new(
                    "invalid_input",
                    "Overview URI parameters are invalid",
                ));
            }
        }
    }
    Ok(Some((limit, cursor, snapshot_revision)))
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CaptureInput {
    title: String,
    summary: String,
    #[serde(default)]
    user_intent: Option<String>,
    #[serde(default)]
    source_excerpt: Option<String>,
    #[serde(default)]
    source_turn_key: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OpenInput {
    operation_id: String,
    lineage_key: String,
    mode: String,
    #[serde(default)]
    capture: Option<CaptureInput>,
    #[serde(default)]
    parent_session_id: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum AppendEvent {
    ProblemDraft {
        statement: String,
        #[serde(default)]
        detail: String,
        #[serde(default)]
        assumptions: Vec<String>,
        #[serde(default, rename = "openQuestions")]
        open_questions: Vec<String>,
        #[serde(default)]
        claims: Vec<Value>,
    },
    SolutionDraft {
        title: String,
        #[serde(default)]
        outcome: String,
        #[serde(default, rename = "nonGoals")]
        non_goals: String,
        #[serde(default, rename = "validationCriteria")]
        validation_criteria: String,
        #[serde(default)]
        claims: Vec<Value>,
    },
    WorkLogCheckpoint {
        summary: String,
        #[serde(default)]
        decisions: Vec<String>,
        #[serde(default)]
        changes: Vec<String>,
        #[serde(default)]
        verification: Vec<String>,
        #[serde(default)]
        validation: Vec<String>,
        #[serde(default, rename = "evidenceRefs")]
        evidence_refs: Vec<String>,
        #[serde(default, rename = "artifactRefs")]
        artifact_refs: Vec<String>,
        #[serde(default)]
        outcomes: Vec<String>,
        #[serde(default)]
        blockers: Vec<String>,
        #[serde(default, rename = "nextSteps")]
        next_steps: Vec<String>,
    },
    ConflictProposal {
        #[serde(rename = "detectedConflict")]
        detected_conflict: String,
        #[serde(default, rename = "affectedRecords")]
        affected_records: Vec<String>,
        #[serde(rename = "proposedResolution")]
        proposed_resolution: String,
        #[serde(default)]
        rationale: String,
        #[serde(default)]
        evidence: Vec<Value>,
        #[serde(default)]
        coverage: String,
    },
    CompletionProposal {
        #[serde(default)]
        outcomes: Vec<String>,
        #[serde(default)]
        verification: Vec<String>,
        #[serde(default, rename = "residualRisks")]
        residual_risks: Vec<String>,
        #[serde(default, rename = "followUps")]
        follow_ups: Vec<String>,
        #[serde(default, rename = "selectedEvidence")]
        selected_evidence: Vec<String>,
    },
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AppendInput {
    operation_id: String,
    session_id: String,
    expected_head_revision: i64,
    #[serde(default)]
    source_turn_key: Option<String>,
    #[serde(default)]
    supersedes_event_id: Option<String>,
    event: AppendEvent,
    #[serde(default)]
    observed_at: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AdvanceInput {
    operation_id: String,
    session_id: String,
    expected_head_revision: i64,
    source_event_id: String,
    action: String,
    #[serde(default)]
    proposed_payload: Value,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionInput {
    session_id: String,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageInput {
    #[serde(default)]
    limit: Option<u64>,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    snapshot_revision: Option<i64>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SearchInput {
    query: String,
    scope: String,
    #[serde(default)]
    target_id: Option<String>,
    #[serde(default)]
    limit: Option<u64>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TopicInput {
    topic_id: String,
    #[serde(default)]
    limit: Option<u64>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EvidenceRefInput {
    evidence_id: String,
    #[serde(default)]
    expected_revision: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EvidenceReadInput {
    items: Vec<EvidenceRefInput>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DraftInput {
    operation_id: String,
    session_id: String,
    completion_event_id: String,
    #[serde(default)]
    draft_id: Option<String>,
    #[serde(default)]
    expected_draft_revision: Option<i64>,
    title: String,
    summary: String,
    body_markdown: String,
    #[serde(default)]
    topic_ids: Vec<String>,
    #[serde(default)]
    evidence_refs: Vec<Value>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PublishInput {
    operation_id: String,
    draft_id: String,
    expected_draft_revision: i64,
    expected_content_hash: String,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeferInput {
    session_id: String,
    completion_revision: i64,
}

fn value(input: impl Serialize) -> Value {
    serde_json::to_value(input).expect("MCP DTO serialization")
}

fn review_decision(response: &Value) -> &str {
    match response["action"].as_str() {
        Some("accept") => match response["content"]["decision"].as_str() {
            Some("accept") => "accept",
            Some("reject") => "reject",
            _ => "cancel",
        },
        Some("decline") | Some("reject") => "reject",
        _ => "cancel",
    }
}

#[derive(Clone)]
pub struct WorkTrackingMcpServer {
    service: WorkTrackingApplicationService,
    connection_id: String,
    tool_router: ToolRouter<Self>,
}

impl WorkTrackingMcpServer {
    pub fn new(service: WorkTrackingApplicationService, connection_id: String) -> Self {
        Self {
            service,
            connection_id,
            tool_router: Self::tool_router(),
        }
    }

    fn result(&self, result: Result<Value, AppError>) -> Result<Json<Value>, String> {
        result.map(Json).map_err(|error| {
            serde_json::to_string(&error).unwrap_or_else(|_| "request_failed".into())
        })
    }

    fn knowledge_review(
        &self,
        action: &str,
        input: &Value,
        modern: bool,
        state: Option<String>,
        responses: Option<Value>,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        if !modern {
            let mut result = CallToolResult::structured(
                json!({"error":{"code":"elicitation_required","message":"Knowledge changes require explicit user review"}}),
            );
            result.is_error = Some(true);
            return Ok(result.into());
        }
        if let Some(state) = state {
            let response = responses
                .as_ref()
                .and_then(|v| v.get("decision"))
                .ok_or_else(|| rmcp::ErrorData::invalid_params("Missing decision", None))?;
            let decision = review_decision(response);
            let result = if action == "draft" {
                self.service
                    .finish_knowledge_draft(&self.connection_id, input, &state, decision)
            } else if action == "withdraw" {
                self.service
                    .finish_withdrawal(&self.connection_id, input, &state, decision)
            } else {
                self.service
                    .finish_publish(&self.connection_id, input, &state, decision)
            }
            .map_err(error_data)?;
            return Ok(CallToolResult::structured(result).into());
        }
        let review = if action == "draft" {
            self.service
                .save_knowledge_draft(&self.connection_id, input)
        } else if action == "withdraw" {
            self.service.begin_withdrawal(&self.connection_id, input)
        } else {
            self.service.begin_publish(&self.connection_id, input)
        }
        .map_err(error_data)?;
        if review["decisionRequired"] != true {
            return Ok(CallToolResult::structured(review).into());
        }
        let schema=serde_json::from_value(json!({"type":"object","additionalProperties":false,"required":["decision"],"properties":{"decision":{"type":"string","enum":["accept","reject"]}}})).map_err(|error|rmcp::ErrorData::internal_error(error.to_string(),None))?;
        let mut requests = BTreeMap::new();
        requests.insert(
            "decision".into(),
            InputRequest::Elicitation(ElicitRequest::new(
                ElicitRequestParams::FormElicitationParams {
                    meta: None,
                    message: format!(
                        "{} this exact Knowledge draft?\n{}",
                        if action == "draft" {
                            "Save privately"
                        } else if action=="withdraw" {
                            "Withdraw from Vault into a recoverable local copy (Completed Work remains unchanged)"
                        } else {
                            "Publish to Vault"
                        },
                        review["preview"]
                    ),
                    requested_schema: schema,
                },
            )),
        );
        Ok(InputRequiredResult::new(
            Some(requests),
            Some(
                review["reviewState"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            ),
        )
        .into())
    }
}

#[tool_router(router = tool_router)]
impl WorkTrackingMcpServer {
    #[tool(
        name = "inbound_work_open",
        description = "Create or resume a private LLM Wiki work session for this chat"
    )]
    async fn inbound_work_open(
        &self,
        Parameters(input): Parameters<OpenInput>,
        context: RequestContext<RoleServer>,
        RequestState(request_state): RequestState,
        ToolInputResponses(input_responses): ToolInputResponses,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let input = value(input);
        if let Some(state) = request_state {
            if context
                .protocol_version()
                .is_none_or(|version| version < ProtocolVersion::V_2026_07_28)
            {
                return Err(rmcp::ErrorData::invalid_params(
                    "Capture requires elicitation support",
                    None,
                ));
            }
            let state = state.as_str();
            let response = input_responses
                .as_ref()
                .and_then(|items| items.get("decision"))
                .ok_or_else(|| rmcp::ErrorData::invalid_params("Missing Capture decision", None))?;
            let decision = review_decision(response);
            return Ok(CallToolResult::structured(
                self.service
                    .finish_open(&self.connection_id, &input, state, decision)
                    .map_err(error_data)?,
            )
            .into());
        }
        let result = self
            .service
            .open(&self.connection_id, &input)
            .map_err(error_data)?;
        if result["decisionRequired"] != true {
            return Ok(CallToolResult::structured(result).into());
        }
        if context
            .protocol_version()
            .is_none_or(|version| version < ProtocolVersion::V_2026_07_28)
        {
            let mut result = CallToolResult::structured(
                json!({"error":{"code":"elicitation_required","message":"Capture requires a client with elicitation support"}}),
            );
            result.is_error = Some(true);
            return Ok(result.into());
        }
        let requested_schema = serde_json::from_value(json!({"type":"object","additionalProperties":false,"required":["decision"],"properties":{"decision":{"type":"string","enum":["accept","reject"]}}})).map_err(|error|rmcp::ErrorData::internal_error(error.to_string(),None))?;
        let mut requests = BTreeMap::new();
        requests.insert(
            "decision".into(),
            InputRequest::Elicitation(ElicitRequest::new(
                ElicitRequestParams::FormElicitationParams {
                    meta: None,
                    message: format!(
                        "Save this exact Capture?\n{}\n{}",
                        result["preview"]["title"].as_str().unwrap_or_default(),
                        result["preview"]["summary"].as_str().unwrap_or_default()
                    ),
                    requested_schema,
                },
            )),
        );
        Ok(InputRequiredResult::new(
            Some(requests),
            Some(
                result["reviewState"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            ),
        )
        .into())
    }

    #[tool(
        name = "inbound_work_append",
        description = "Durably append a bounded work proposal or checkpoint without advancing workflow state"
    )]
    async fn inbound_work_append(
        &self,
        Parameters(input): Parameters<AppendInput>,
    ) -> Result<Json<Value>, String> {
        let input = value(input);
        self.result(self.service.append(&self.connection_id, &input))
    }

    #[tool(
        name = "inbound_work_advance",
        description = "Review and decide one exact governed workflow action through MCP elicitation"
    )]
    async fn inbound_work_advance(
        &self,
        Parameters(input): Parameters<AdvanceInput>,
        context: RequestContext<RoleServer>,
        RequestState(request_state): RequestState,
        ToolInputResponses(input_responses): ToolInputResponses,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let input = value(input);
        if context
            .protocol_version()
            .is_none_or(|version| version < ProtocolVersion::V_2026_07_28)
        {
            let mut result = CallToolResult::structured(
                json!({"error":{"code":"elicitation_required","message":"This action requires an MCP 2026-07-28 client with elicitation support"}}),
            );
            result.is_error = Some(true);
            return Ok(result.into());
        }
        if let Some(state) = request_state {
            let response = input_responses
                .as_ref()
                .and_then(|items| items.get("decision"))
                .ok_or_else(|| rmcp::ErrorData::invalid_params("Missing review response", None))?;
            let decision = review_decision(response);
            let result = self
                .service
                .finish_advance(&self.connection_id, &state, &input, decision)
                .map_err(error_data)?;
            return Ok(CallToolResult::structured(result).into());
        }
        let state = self
            .service
            .begin_advance(&self.connection_id, &input)
            .map_err(error_data)?;
        let action = input
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("workflow change");
        let mut requests = BTreeMap::new();
        let requested_schema=serde_json::from_value(json!({
            "type":"object","additionalProperties":false,"required":["decision"],
            "properties":{"decision":{"type":"string","enum":["accept","reject"]},"note":{"type":"string","maxLength":1000}}
        })).map_err(|error|rmcp::ErrorData::internal_error(error.to_string(),None))?;
        let target = self
            .service
            .review_target(&self.connection_id, &state)
            .map_err(error_data)?;
        requests.insert("decision".into(),InputRequest::Elicitation(ElicitRequest::new(ElicitRequestParams::FormElicitationParams {
            meta:None,message:format!("Review the exact LLM Wiki action: {action}.\nProposal: {}\nTarget: {}\nAccept or reject it in this chat.",input["proposedPayload"],target),requested_schema,
        })));
        Ok(InputRequiredResult::new(Some(requests), Some(state)).into())
    }

    #[tool(
        name = "inbound_work_session_read",
        description = "Read fresh bounded state for an owned work session"
    )]
    async fn inbound_work_session_read(
        &self,
        Parameters(input): Parameters<SessionInput>,
    ) -> Result<Json<Value>, String> {
        self.result(self.service.session(&self.connection_id, &input.session_id))
    }

    #[tool(
        name = "workbench_current",
        description = "Read the current bounded Workbench orientation when this connection has scope"
    )]
    async fn workbench_current(
        &self,
        Parameters(input): Parameters<PageInput>,
    ) -> Result<Json<Value>, String> {
        self.result(
            self.service
                .current_workbench(&self.connection_id, input.limit.unwrap_or(10) as usize),
        )
    }

    #[tool(
        name = "workbench_overview",
        description = "Read a paginated summary of Workbench state when this connection has scope"
    )]
    async fn workbench_overview(
        &self,
        Parameters(input): Parameters<PageInput>,
    ) -> Result<Json<Value>, String> {
        self.result(self.service.overview(
            &self.connection_id,
            input.limit.unwrap_or(20) as usize,
            input.cursor.as_deref(),
            input.snapshot_revision,
        ))
    }

    #[tool(
        name = "workbench_topic",
        description = "Read bounded Workbench context for one explicit topic scope"
    )]
    async fn workbench_topic(
        &self,
        Parameters(input): Parameters<TopicInput>,
    ) -> Result<Json<Value>, String> {
        self.result(self.service.topic(
            &self.connection_id,
            &input.topic_id,
            input.limit.unwrap_or(20) as usize,
        ))
    }

    #[tool(
        name = "vault_search_lexical",
        description = "Search visible Vault evidence by exact terms; does not invoke a model"
    )]
    async fn vault_search_lexical(
        &self,
        Parameters(input): Parameters<SearchInput>,
    ) -> Result<Json<Value>, String> {
        self.result(self.service.lexical_search(
            &self.connection_id,
            &input.scope,
            input.target_id.as_deref().unwrap_or(""),
            &input.query,
            input.limit.unwrap_or(10) as usize,
        ))
    }

    #[tool(
        name = "vault_search_semantic",
        description = "Search visible Vault evidence semantically; does not invoke a model"
    )]
    async fn vault_search_semantic(
        &self,
        Parameters(input): Parameters<SearchInput>,
    ) -> Result<Json<Value>, String> {
        self.result(self.service.semantic_search(
            &self.connection_id,
            &input.scope,
            input.target_id.as_deref().unwrap_or(""),
            &input.query,
            input.limit.unwrap_or(5) as usize,
        ))
    }

    #[tool(
        name = "vault_evidence_read",
        description = "Read bounded evidence using server-generated evidence IDs, never filesystem paths"
    )]
    async fn vault_evidence_read(
        &self,
        Parameters(input): Parameters<EvidenceReadInput>,
    ) -> Result<Json<Value>, String> {
        let input = value(input);
        let items = input["items"].as_array().expect("items serialized");
        self.result(self.service.evidence_read(&self.connection_id, items))
    }

    #[tool(
        name = "knowledge_draft_save",
        description = "Save a private append-versioned Knowledge draft from completed work; never publishes"
    )]
    async fn knowledge_draft_save(
        &self,
        Parameters(input): Parameters<DraftInput>,
        context: RequestContext<RoleServer>,
        RequestState(state): RequestState,
        ToolInputResponses(responses): ToolInputResponses,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let input = value(input);
        self.knowledge_review(
            "draft",
            &input,
            context
                .protocol_version()
                .is_some_and(|v| v >= ProtocolVersion::V_2026_07_28),
            state,
            responses.map(|v| serde_json::to_value(v).unwrap_or(Value::Null)),
        )
    }

    #[tool(
        name = "knowledge_publish",
        description = "Publish one exact reviewed Knowledge draft revision and hash without content edits"
    )]
    async fn knowledge_publish(
        &self,
        Parameters(input): Parameters<PublishInput>,
        context: RequestContext<RoleServer>,
        RequestState(state): RequestState,
        ToolInputResponses(responses): ToolInputResponses,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let input = value(input);
        self.knowledge_review(
            "publish",
            &input,
            context
                .protocol_version()
                .is_some_and(|v| v >= ProtocolVersion::V_2026_07_28),
            state,
            responses.map(|v| serde_json::to_value(v).unwrap_or(Value::Null)),
        )
    }

    #[tool(
        name = "knowledge_publication_defer",
        description = "Defer the one-time Knowledge publication offer for an exact completion revision"
    )]
    async fn knowledge_publication_defer(
        &self,
        Parameters(input): Parameters<DeferInput>,
    ) -> Result<Json<Value>, String> {
        self.result(self.service.defer_publication(
            &self.connection_id,
            &input.session_id,
            input.completion_revision,
        ))
    }

    #[tool(
        name = "knowledge_publication_withdraw",
        description = "Review withdrawing an exact publication into a recoverable local copy; preserves Completed Work and blocks external changes"
    )]
    async fn knowledge_publication_withdraw(
        &self,
        Parameters(input): Parameters<PublishInput>,
        context: RequestContext<RoleServer>,
        RequestState(state): RequestState,
        ToolInputResponses(responses): ToolInputResponses,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        self.knowledge_review(
            "withdraw",
            &value(input),
            context
                .protocol_version()
                .is_some_and(|v| v >= ProtocolVersion::V_2026_07_28),
            state,
            responses.map(value),
        )
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for WorkTrackingMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info=ServerInfo::new(ServerCapabilities::builder().enable_tools().enable_resources().build())
            .with_server_info(Implementation::new("llm-wiki", env!("CARGO_PKG_VERSION")))
            .with_instructions("Track work in LLM Wiki without leaving chat. Completion and Knowledge publication are separate decisions.");
        info.protocol_version = ProtocolVersion::V_2026_07_28;
        info
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let scopes = self
            .service
            .scopes(&self.connection_id)
            .map_err(error_data)?;
        let mut resources = self
            .service
            .owned_sessions(&self.connection_id)
            .map_err(error_data)?
            .into_iter()
            .map(|(id, title)| {
                Resource::new(format!("llm-wiki://work-session/{id}"), title)
                    .with_mime_type("application/json")
            })
            .collect::<Vec<_>>();
        if scopes.iter().any(|scope| scope == "workbench:current:read") {
            resources.push(
                Resource::new("llm-wiki://workbench/current", "Current Workbench")
                    .with_description("Fresh bounded orientation for active work")
                    .with_mime_type("application/json"),
            );
        }
        if scopes
            .iter()
            .any(|scope| scope == "workbench:overview:read")
        {
            resources.push(
                Resource::new(WORKBENCH_OVERVIEW_URI, "Workbench overview")
                    .with_description("Paginated whole-Workbench summary")
                    .with_mime_type("application/json"),
            );
        }
        Ok(ListResourcesResult::with_all_items(resources)
            .with_ttl_ms(0)
            .with_cache_scope(CacheScope::Private))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, rmcp::ErrorData> {
        let scopes = self
            .service
            .scopes(&self.connection_id)
            .map_err(error_data)?;
        let templates = if scopes
            .iter()
            .any(|scope| scope == "workbench:overview:read")
        {
            vec![
                ResourceTemplate::new(WORKBENCH_OVERVIEW_TEMPLATE, "Workbench overview pages")
                    .with_description("A stable, paginated whole-Workbench summary")
                    .with_mime_type("application/json"),
            ]
        } else {
            Vec::new()
        };
        Ok(ListResourceTemplatesResult::with_all_items(templates)
            .with_ttl_ms(0)
            .with_cache_scope(CacheScope::Private))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, rmcp::ErrorData> {
        let uri = request.uri;
        let value = if uri == "llm-wiki://workbench/current" {
            self.service.current_workbench(&self.connection_id, 10)
        } else if let Some((limit, cursor, snapshot_revision)) =
            overview_resource_page(&uri).map_err(error_data)?
        {
            self.service
                .overview(&self.connection_id, limit, cursor, snapshot_revision)
        } else if let Some(id) = uri.strip_prefix("llm-wiki://topic/") {
            self.service.topic(&self.connection_id, id, 50)
        } else if let Some(id) = uri
            .strip_prefix("llm-wiki://work-session/")
            .and_then(|tail| tail.split('?').next())
        {
            self.service.session(&self.connection_id, id)
        } else {
            return Err(rmcp::ErrorData::resource_not_found(
                "Resource is unavailable",
                None,
            ));
        }
        .map_err(error_data)?;
        let text = serde_json::to_string(&value)
            .map_err(|error| rmcp::ErrorData::internal_error(error.to_string(), None))?;
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(text, uri).with_mime_type("application/json")
        ])
        .into())
    }
}

pub async fn serve_transport<T, E, A>(
    service: WorkTrackingApplicationService,
    connection_id: String,
    transport: T,
) -> Result<(), String>
where
    T: IntoTransport<RoleServer, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let server = WorkTrackingMcpServer::new(service, connection_id);
    let running = server
        .serve(transport)
        .await
        .map_err(|error| format!("MCP startup failed: {error}"))?;
    running
        .waiting()
        .await
        .map(|_| ())
        .map_err(|error| format!("MCP transport failed: {error}"))
}

pub fn safe_error(error: AppError) -> Value {
    json!({"error":error})
}

fn error_data(error: AppError) -> rmcp::ErrorData {
    rmcp::ErrorData::invalid_params(serde_json::to_string(&error).unwrap_or(error.code), None)
}

#[cfg(test)]
mod decision_tests {
    use super::*;
    #[test]
    fn outer_cancel_or_decline_cannot_be_overridden_by_model_content() {
        assert_eq!(
            review_decision(&json!({"action":"cancel","content":{"decision":"accept"}})),
            "cancel"
        );
        assert_eq!(
            review_decision(&json!({"action":"decline","content":{"decision":"accept"}})),
            "reject"
        );
        assert_ne!(
            review_decision(&json!({"content":{"decision":"accept"}})),
            "accept"
        );
        assert_eq!(
            review_decision(&json!({"action":"accept","content":{"decision":"accept"}})),
            "accept"
        );
    }
}
