from __future__ import annotations

from pydantic import BaseModel, Field


class CaptureIn(BaseModel):
    text: str = Field(min_length=1, max_length=20_000)


class ProblemIn(BaseModel):
    statement: str | None = Field(default=None, max_length=20_000)
    detail: str = Field(default="", max_length=20_000)
    localized_versions: dict[str, dict[str, str]] | None = None


class FeatureIn(BaseModel):
    title: str = Field(min_length=1, max_length=500)
    outcome: str = Field(min_length=1, max_length=10_000)
    non_goals: str = Field(default="", max_length=10_000)
    validation_criteria: str = Field(min_length=1, max_length=10_000)
    localized_versions: dict[str, dict[str, str]] | None = None


class ConflictAddressIn(BaseModel):
    basis: str
    disposition: str | None = None
    summary: str = Field(default="", max_length=10_000)
    evidence_source_type: str = Field(default="", max_length=100)
    evidence_source_id: str = Field(default="", max_length=200)
    conflict_report_id: str = Field(default="", max_length=200)


class ConflictIn(BaseModel):
    state: str
    citation: str = Field(default="", max_length=10_000)
    address: ConflictAddressIn | None = None


class ConflictResolutionIn(BaseModel):
    conflict_id: str = Field(min_length=1, max_length=200)
    action: str = Field(pattern="^(apply_recommendation|accept_conflict)$")
    rationale: str = Field(default="", max_length=10_000)


class ConflictResolutionsIn(BaseModel):
    resolutions: list[ConflictResolutionIn] = Field(min_length=1, max_length=50)


class LineageRegenerateIn(BaseModel):
    include_inference: bool = True


class LineageCorrectionIn(BaseModel):
    text: str = Field(min_length=1, max_length=20_000)
    reason: str = Field(default="", max_length=10_000)
    current_revision_id: str = Field(default="", max_length=200)


class SolutionProgressIn(BaseModel):
    body: str = Field(default="", max_length=20_000)
    image_data: str = Field(default="", max_length=12_000_000)
    image_media_type: str = Field(default="", max_length=100)


class SolutionCommentIn(BaseModel):
    body: str = Field(min_length=1, max_length=10_000)


class SolutionChecklistIn(BaseModel):
    body: str = Field(min_length=1, max_length=10_000)
    checked: bool = False


class ImportanceIn(BaseModel):
    alignment: int = Field(ge=0, le=5)
    impact: int = Field(ge=0, le=5)
    urgency: int = Field(ge=0, le=5)
    leverage: int = Field(ge=0, le=5)
    evidence: str = Field(min_length=1, max_length=10_000)


class GoalIn(BaseModel):
    title: str = Field(min_length=1, max_length=500)
    description: str = Field(default="", max_length=10_000)


class CompletionIn(BaseModel):
    evidence: str = Field(min_length=1, max_length=20_000)
    report: str = Field(min_length=1, max_length=20_000)
    no_update_reason: str = Field(default="", max_length=10_000)


class PatchIn(BaseModel):
    path: str = Field(min_length=1)
    operation: str
    heading: str = Field(min_length=1)
    content: str = Field(min_length=1, max_length=50_000)


class ProviderConfigIn(BaseModel):
    base_url: str
    model: str = ""
    advanced_model: str = ""
    api_key: str | None = Field(default=None, min_length=1)
    advanced_tasks: dict[str, bool] = Field(default_factory=dict)
    report_language: str | None = Field(default=None, pattern="^(ko|en)$")
    async_worker_count: int = Field(default=2, ge=1, le=32)


class WorkbenchCategoryIn(BaseModel):
    entity_type: str
    entity_id: str
    category: str = Field(min_length=1, max_length=80)


class WorkbenchImportanceIn(BaseModel):
    entity_type: str
    entity_id: str
    important: bool


class FeatureStageIn(BaseModel):
    state: str


class ProblemCompletionIn(BaseModel):
    """A user may complete a partially resolved problem and record why."""

    reason: str = Field(default="", max_length=10_000)
    review_id: str = Field(default="", max_length=200)


class EnrichIn(BaseModel):
    statement: str = Field(min_length=1, max_length=20_000)
    citations: list[str] = Field(default_factory=list)


class ChatIn(BaseModel):
    message: str = Field(min_length=1, max_length=10_000)


class ManualUpdateIn(BaseModel):
    title: str = Field(min_length=1, max_length=10_000)
    detail: str = Field(default="", max_length=10_000)
    localized_versions: dict[str, dict[str, str]] | None = None


class LocaleIn(BaseModel):
    locale: str


class LocalizedSupplementIn(BaseModel):
    locale: str
    fields: dict[str, str]


class TransitionIn(BaseModel):
    """A menu-only Workflow Transition with its required input form fields."""

    transition_id: str = Field(min_length=1, max_length=100)
    fields: dict[str, object] = Field(default_factory=dict)
