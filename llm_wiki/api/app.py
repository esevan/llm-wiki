from __future__ import annotations

from contextlib import asynccontextmanager
from datetime import date
import asyncio
import json
import sqlite3
import threading
from pathlib import Path

from fastapi import FastAPI, HTTPException, Query, Request
from fastapi.responses import FileResponse, JSONResponse, Response, StreamingResponse
from fastapi.responses import PlainTextResponse
from pydantic import BaseModel, Field

from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.vault import MarkdownVaultAdapter
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError, TRANSITIONS, available_transitions
from llm_wiki.services.patches import PatchConflict, SectionPatch, apply_reviewed_patch, propose_section_patch
from llm_wiki.services.patches import digest
from llm_wiki.services.provider import OpenAICompatibleProvider
from llm_wiki.services.ai import AIEnrichmentEngine
from llm_wiki.services.graphs import enrich_problem_graph
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.conversation import bilingual_draft_prompt, bilingual_refinement_prompt, draft_prompt, refinement_focus_prompt, refinement_prompt, system_prompt
from llm_wiki.services.localization import (
    KnowledgeTranslationCache,
    VaultKnowledgeTranslationCache,
    LocaleSettings,
    SUPPORTED_LOCALES,
    load_locale_resources,
    knowledge_translation_blocks,
    localize_descriptor,
    normalize_locale,
    response_language_instruction,
)
from llm_wiki.services.lineage import readable_report_context, report_context, validate_inference_payload
from llm_wiki.services.conflict_review import ConflictReviewManager


CHAT_RESPONSE_CHARACTER_LIMIT = 1_200


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
    alignment: int = Field(ge=0, le=5); impact: int = Field(ge=0, le=5); urgency: int = Field(ge=0, le=5); leverage: int = Field(ge=0, le=5)
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
    """A human may complete a partially resolved problem and record why."""
    reason: str = Field(default="", max_length=10_000)
    review_id: str = Field(default="", max_length=200)


class EnrichIn(BaseModel):
    statement: str = Field(min_length=1, max_length=20_000)
    citations: list[str] = []


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



_DRAFT_FIELDS = {
    "captures": ("title", "detail"),
    "problems": ("title", "outcome", "non_goals", "validation_criteria"),
}


def validate_draft(entity_type: str, value: dict[str, object]) -> dict[str, str]:
    """Reject incomplete model output before it reaches the human-review UI."""
    fields = _DRAFT_FIELDS.get(entity_type)
    if not fields:
        raise ValueError(f"Unknown workflow draft: {entity_type}")
    result = {field: str(value.get(field, "")).strip() for field in fields}
    missing = [field for field in fields if not result[field]]
    if missing:
        raise ValueError(f"AI draft is missing required fields: {', '.join(missing)}")
    return result


def validate_bilingual_draft(entity_type: str, value: dict[str, object]) -> dict[str, dict[str, str]]:
    """Validate both durable variants before either can be applied."""
    if set(value) != set(SUPPORTED_LOCALES):
        raise ValueError("AI draft must contain complete Korean and English versions")
    return {locale: validate_draft(entity_type, dict(value[locale])) for locale in SUPPORTED_LOCALES}


def validate_bilingual_image_summary(value: dict[str, object]) -> dict[str, dict[str, str]]:
    """Require complete non-empty KO+EN summaries before changing evidence."""
    if set(value) != set(SUPPORTED_LOCALES):
        raise ValueError("Image Summary must contain complete Korean and English versions")
    versions: dict[str, dict[str, str]] = {}
    for locale in SUPPORTED_LOCALES:
        payload = value[locale]
        if not isinstance(payload, dict):
            raise ValueError(f"Image Summary {locale} version must be an object")
        summary = str(payload.get("summary", "")).strip()
        if not summary:
            raise ValueError(f"Image Summary {locale} version cannot be empty")
        versions[locale] = {"image_summary": summary}
    return versions


def validate_refinement(entity_type: str, value: dict[str, object]) -> dict[str, str]:
    fields = {"captures": ("title",), "problems": ("title", "detail"), "features": ("title", "detail")}.get(entity_type)
    if not fields:
        raise ValueError(f"Unknown workflow refinement: {entity_type}")
    result = {field: str(value.get(field, "")).strip() for field in fields}
    missing = [field for field in fields if not result[field]]
    if missing:
        raise ValueError(f"AI refinement is missing required fields: {', '.join(missing)}")
    return result


def create_app(vault_path: Path, db_path: Path) -> FastAPI:
    vault = MarkdownVaultAdapter(vault_path)
    retrieval = RetrievalEngine(db_path, vault)
    workflow = WorkflowEngine(retrieval.db)
    provider_settings = ProviderSettings(retrieval.db)
    locale_settings = LocaleSettings(retrieval.db)
    legacy_knowledge_cache = KnowledgeTranslationCache(retrieval.db)
    knowledge_cache = VaultKnowledgeTranslationCache(vault, legacy_knowledge_cache)
    knowledge_translation_lock = threading.Lock()
    knowledge_translation_jobs: dict[str, threading.Event] = {}
    knowledge_translation_cancelled_ids: set[str] = set()

    def conflict_provider(strong: bool) -> OpenAICompatibleProvider:
        base_url, api_key, model = provider_settings.credentials("conflict_review" if strong else None)
        return OpenAICompatibleProvider(base_url, api_key, model)

    conflict_reviews = ConflictReviewManager(retrieval, workflow, conflict_provider)

    def refresh_embeddings_background() -> None:
        if app.state.semantic_running:
            return
        def refresh() -> None:
            try:
                retrieval.refresh_embeddings()
            except Exception:
                pass  # semantic support is optional; coverage exposes failure to callers
            finally:
                app.state.semantic_running = False
        app.state.semantic_running = True
        threading.Thread(target=refresh, name="llm-wiki-semantic", daemon=True).start()

    def knowledge_translation_cancelled(request_id: str) -> bool:
        with knowledge_translation_lock:
            event = knowledge_translation_jobs.get(request_id)
            return request_id in knowledge_translation_cancelled_ids or bool(event and event.is_set())

    def cancel_knowledge_translation(request_id: str) -> None:
        with knowledge_translation_lock:
            knowledge_translation_cancelled_ids.add(request_id)
            event = knowledge_translation_jobs.get(request_id)
            if event:
                event.set()

    async def watch_vault() -> None:
        from watchfiles import awatch  # keep file-watching out of local request hot paths
        async for _changes in awatch(vault.root):
            knowledge_cache.cleanup()
            await asyncio.to_thread(retrieval.index_changed)
            refresh_embeddings_background()
            async with app.state.index_condition:
                app.state.index_revision += 1
                app.state.index_condition.notify_all()

    @asynccontextmanager
    async def lifespan(app: FastAPI):
        # Structural indexing is intentionally separate from app construction and AI-free.
        knowledge_cache.cleanup()
        retrieval.index_changed()
        refresh_embeddings_background()
        watcher = asyncio.create_task(watch_vault())
        yield
        watcher.cancel()
        try:
            await watcher
        except asyncio.CancelledError:
            pass
        retrieval.db.close()

    app = FastAPI(title="LLM Wiki", lifespan=lifespan)
    app.state.retrieval = retrieval
    app.state.workflow = workflow
    app.state.index_revision = 0
    app.state.index_condition = asyncio.Condition()
    app.state.semantic_running = False
    app.state.provider_settings = provider_settings
    app.state.locale_settings = locale_settings
    app.state.knowledge_cache = knowledge_cache
    app.state.knowledge_translation_cancelled = knowledge_translation_cancelled
    app.state.conflict_reviews = conflict_reviews

    @app.middleware("http")
    async def bind_request_locale(request: Request, call_next):
        raw = request.headers.get("X-LLM-Wiki-Locale")
        if raw and raw.strip().lower().replace("_", "-").split("-", 1)[0] not in SUPPORTED_LOCALES:
            return JSONResponse({"detail": "Unsupported locale", "code": "unsupported_locale"}, status_code=400)
        saved = locale_settings.get().get("locale", "en")
        request.state.locale = normalize_locale(raw, str(saved)) if raw else str(saved)
        return await call_next(request)

    @app.get("/api/settings/locale")
    def get_locale(browser_locale: str = "") -> JSONResponse:
        return JSONResponse(
            locale_settings.get(browser_locale),
            headers={"Cache-Control": "no-store"},
        )

    @app.put("/api/settings/locale")
    def save_locale(data: LocaleIn) -> dict[str, object]:
        try:
            return locale_settings.save(data.locale)
        except ValueError as error:
            raise HTTPException(400, detail={"code": "unsupported_locale", "message": str(error)}) from error

    @app.get("/api/i18n/{locale}")
    def i18n_resource(locale: str) -> JSONResponse:
        try:
            return JSONResponse(load_locale_resources(locale), headers={"Cache-Control": "no-store"})
        except ValueError as error:
            raise HTTPException(404, str(error)) from error

    @app.get("/api/health")
    def health() -> dict[str, object]:
        return {"status": "ok", "vault": str(vault.root), **retrieval.status()}

    @app.post("/api/index")
    def index() -> dict[str, int | float]:
        return retrieval.index_changed()

    @app.get("/api/events")
    async def events() -> StreamingResponse:
        async def stream():
            revision = app.state.index_revision
            yield "event: ready\ndata: indexed\n\n"
            while True:
                async with app.state.index_condition:
                    await app.state.index_condition.wait_for(lambda: app.state.index_revision != revision)
                    revision = app.state.index_revision
                yield f"event: indexed\ndata: {revision}\n\n"
        return StreamingResponse(stream(), media_type="text/event-stream", headers={"Cache-Control": "no-cache"})

    @app.get("/api/search")
    def search(q: str = Query(min_length=1, max_length=500), limit: int = Query(20, ge=1, le=100), offset: int = Query(0, ge=0), semantic: bool = False) -> dict[str, object]:
        if semantic and not app.state.semantic_running and retrieval.status()["semantic_ready"] < retrieval.status()["documents"]:
            def refresh() -> None:
                try:
                    retrieval.refresh_embeddings()
                finally:
                    app.state.semantic_running = False
            app.state.semantic_running = True
            threading.Thread(target=refresh, name="llm-wiki-semantic", daemon=True).start()
        return {"query": q, "offset": offset, "results": [result.__dict__ for result in retrieval.search(q, limit, semantic, offset)]}

    @app.post("/api/captures", status_code=201)
    def capture(data: CaptureIn) -> dict[str, str]:
        return {"id": workflow.capture(data.text), "text": data.text}

    @app.get("/api/board")
    def board(request: Request) -> dict[str, list[dict[str, object]]]:
        return workflow.board(request.state.locale)

    @app.get("/api/problems/{problem_id}/record")
    def problem_record(problem_id: str) -> dict[str, str]:
        try:
            return workflow.problem_record(problem_id)
        except WorkflowError as error:
            raise HTTPException(404, str(error)) from error

    @app.get("/api/transitions")
    def list_transitions(request: Request) -> dict[str, list[dict[str, object]]]:
        """Return all Workflow Transitions and their required input form definitions."""
        return {"transitions": localize_descriptor(TRANSITIONS, request.state.locale)}

    @app.get("/api/transitions/{entity_type}/{entity_id}")
    def list_transitions_for_item(entity_type: str, entity_id: str, request: Request) -> dict[str, list[dict[str, object]]]:
        """Return transitions available for a specific item, filtered by its current state."""
        try:
            entity = None
            if entity_type in {"captures", "problems", "features"}:
                row = workflow.db.execute(f"SELECT * FROM {entity_type} WHERE id=?", (entity_id,)).fetchone()
                if row:
                    entity = dict(row)
            if entity is None:
                raise WorkflowError("Item not found")
            return {"transitions": localize_descriptor(available_transitions(entity_type, entity), request.state.locale)}
        except WorkflowError as error:
            raise HTTPException(404, str(error)) from error

    @app.post("/api/transitions/{entity_type}/{entity_id}", status_code=200)
    def apply_transition(entity_type: str, entity_id: str, data: TransitionIn) -> dict[str, object]:
        """Apply a menu-only Workflow Transition with its required input form.

        This is the single entry point for all transitions, including the
        skip-conflict-check and complete-without-report paths.
        """
        try:
            result = workflow.apply_transition(data.transition_id, entity_type, entity_id, data.fields)
            # The solution_to_completed transition completes the Problem and writes
            # the completion playbook, so the frontend gets the archive path back.
            if data.transition_id == "solution_to_completed" and "problem_id" in result and not result.get("note_skipped"):
                try:
                    playbook = write_completion_playbook(str(result["problem_id"]), refresh_lineage=True)
                    result["playbook"] = playbook
                except (WorkflowError, OSError) as error:
                    # The Problem is already completed; the playbook failure is non-fatal.
                    result["playbook_error"] = str(error)
            return result
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error
        except OSError as error:
            raise HTTPException(409, str(error)) from error

    @app.post("/api/workbench/organize")
    def organize_workbench(request: Request) -> dict[str, int]:
        """Use the configured model only when the human explicitly asks to organize."""
        try:
            board = workflow.board(request.state.locale)
            items = [
                {
                    "entity_type": entity_type,
                    "entity_id": item["id"],
                    "title": item.get("statement") or item.get("title") or item.get("outcome") or item.get("text"),
                    "state": item.get("state", "inbox"),
                    "current_category": item.get("category", ""),
                }
                for entity_type in ("captures", "problems", "features")
                for item in board[entity_type]
            ]
            if not items:
                return {"organized": 0}
            base_url, api_key, model = provider_settings.credentials("workbench_organization")
            response = OpenAICompatibleProvider(base_url, api_key, model).complete_json(
                [
                    {"role": "system", "content": "You organize a personal workbench. Return JSON only: {\"entries\":[{\"entity_type\":string,\"entity_id\":string,\"category\":string,\"attention_rank\":integer 0-100,\"rationale\":string}]}. Keep category names short, reuse existing categories when appropriate, prioritize urgent unresolved decisions and approved active work. Never change workflow states."},
                    {"role": "system", "content": response_language_instruction(request.state.locale)},
                    {"role": "user", "content": json.dumps({"items": items}, ensure_ascii=False)},
                ],
                "workbench organization",
            )
            return {"organized": workflow.apply_ai_organization(response.get("entries"))}
        except (WorkflowError, ValueError, OSError) as error:
            raise HTTPException(502, f"AI organization failed: {error}") from error

    @app.put("/api/workbench/category", status_code=204)
    def update_workbench_category(data: WorkbenchCategoryIn) -> None:
        try:
            workflow.set_workbench_category(data.entity_type, data.entity_id, data.category)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.put("/api/workbench/importance", status_code=204)
    def update_workbench_importance(data: WorkbenchImportanceIn) -> None:
        try:
            workflow.set_workbench_importance(data.entity_type, data.entity_id, data.important)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.get("/api/workbench/recent-archive")
    def recent_archive(limit: int = Query(default=5, ge=1, le=50)) -> dict[str, list[dict[str, str]]]:
        rows = retrieval.db.execute(
            """SELECT path, title FROM documents
               WHERE path LIKE '%/90. Archive/%' OR path LIKE '90. Archive/%'
               ORDER BY modified_ns DESC LIMIT ?""",
            (limit,),
        ).fetchall()
        return {"documents": [dict(row) for row in rows]}

    @app.get("/api/workbench/completed-solutions")
    def completed_solutions(request: Request, limit: int = Query(default=5, ge=1, le=50)) -> dict[str, list[dict[str, object]]]:
        solutions = workflow.recent_completed_solutions(limit, request.state.locale)
        for solution in solutions:
            path = solution.get("completion_playbook_path", "")
            solution["archive_status"] = "available" if path and (vault.root / path).is_file() else "missing"
        return {"solutions": solutions}

    @app.post("/api/features/{feature_id}/follow-up-problem", status_code=201)
    def create_follow_up_problem(feature_id: str) -> dict[str, str]:
        try:
            return workflow.create_follow_up_problem(feature_id)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.get("/api/items/{entity_type}/{entity_id}")
    def item_record(entity_type: str, entity_id: str) -> dict[str, object]:
        try:
            return workflow.current_item_detail(entity_type, entity_id)
        except WorkflowError as error:
            raise HTTPException(404, str(error)) from error

    @app.delete("/api/items/{entity_type}/{entity_id}", status_code=204)
    def delete_item(entity_type: str, entity_id: str) -> None:
        try:
            workflow.delete(entity_type, entity_id)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/items/{entity_type}/{entity_id}/restore", status_code=204)
    def restore_item(entity_type: str, entity_id: str) -> None:
        try:
            workflow.restore(entity_type, entity_id)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.put("/api/items/{entity_type}/{entity_id}", status_code=204)
    def manual_update(entity_type: str, entity_id: str, data: ManualUpdateIn) -> None:
        try:
            workflow.update_manual(entity_type, entity_id, data.title, data.detail, data.localized_versions)
        except (WorkflowError, ValueError) as error:
            workflow.db.rollback()
            raise HTTPException(400, str(error)) from error

    @app.put("/api/items/{entity_type}/{entity_id}/localizations", status_code=204)
    def supplement_localization(entity_type: str, entity_id: str, data: LocalizedSupplementIn) -> None:
        try:
            row = workflow.db.execute(f"SELECT 1 FROM {entity_type} WHERE id=?", (entity_id,)).fetchone() if entity_type in {"problems", "features"} else None
            if not row:
                raise WorkflowError("Item not found")
            workflow.localized.supplement(entity_type, entity_id, data.locale, data.fields)
            workflow.db.commit()
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/{entity_type}/{entity_id}/chat")
    def chat(entity_type: str, entity_id: str, data: ChatIn, request: Request) -> StreamingResponse:
        try:
            context = workflow.context_for(entity_type, entity_id, request.state.locale)
            task = {"captures": "capture_assistance", "problems": "problem_assistance", "features": "solution_assistance"}.get(entity_type)
            base_url, api_key, model = provider_settings.credentials(task)
            provider = OpenAICompatibleProvider(base_url, api_key, model)
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

        def stream():
            output: list[str] = []
            sent = 0
            assessment = workflow.refinement_structure_assessment(entity_type, entity_id) if entity_type in {"problems", "features"} else {}
            messages = [
                {"role": "system", "content": system_prompt(entity_type)},
                {"role": "system", "content": response_language_instruction(request.state.locale)},
                {"role": "system", "content": f"Current {context['type']}: {context['title']}\nKnown detail: {context['detail']}"},
                *([{"role": "system", "content": refinement_focus_prompt(assessment)}] if assessment else []),
                *workflow.chat_history(entity_type, entity_id),
                {"role": "user", "content": data.message},
            ]
            try:
                for text in provider.stream(messages):
                    remaining = CHAT_RESPONSE_CHARACTER_LIMIT - sent
                    if remaining <= 0:
                        break
                    compact = text[:remaining]
                    sent += len(compact)
                    output.append(compact)
                    yield f"data: {compact.replace(chr(10), ' ')}\n\n"
            except (OSError, ValueError) as error:
                # A stream already has a 200 response. Send a useful SSE error rather than
                # crashing the response and making the browser report a network failure.
                yield f"event: error\ndata: AI service is temporarily unavailable ({error}). Please try again.\n\n"
                return
            workflow.record_ai_run(entity_type, entity_id, "workflow_chat", data.message, "".join(output))
            yield "event: done\ndata: done\n\n"
        return StreamingResponse(stream(), media_type="text/event-stream", headers={"Cache-Control": "no-cache"})

    @app.post("/api/{entity_type}/{entity_id}/next-chat")
    def next_chat(entity_type: str, entity_id: str, data: ChatIn, request: Request) -> StreamingResponse:
        """Collect the required information for the next stage before drafting it."""
        next_stage = {"captures": "problems", "problems": "features"}.get(entity_type)
        if not next_stage:
            raise HTTPException(400, "Solutions do not have a next workflow stage")
        try:
            context = workflow.context_for(entity_type, entity_id, request.state.locale)
            base_url, api_key, model = provider_settings.credentials("problem_drafting" if next_stage == "problems" else "solution_drafting")
            provider = OpenAICompatibleProvider(base_url, api_key, model)
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

        def stream():
            output: list[str] = []
            sent = 0
            messages = [
                {"role": "system", "content": system_prompt(next_stage)},
                {"role": "system", "content": response_language_instruction(request.state.locale)},
                {"role": "system", "content": f"Source {context['type']}: {context['title']}\nKnown detail: {context['detail']}"},
                *workflow.chat_history(entity_type, entity_id),
                {"role": "user", "content": data.message},
            ]
            try:
                for text in provider.stream(messages):
                    remaining = CHAT_RESPONSE_CHARACTER_LIMIT - sent
                    if remaining <= 0:
                        break
                    compact = text[:remaining]
                    sent += len(compact)
                    output.append(compact)
                    yield f"data: {compact.replace(chr(10), ' ')}\n\n"
            except (OSError, ValueError) as error:
                yield f"event: error\ndata: AI service is temporarily unavailable ({error}). Please try again.\n\n"
                return
            workflow.record_ai_run(entity_type, entity_id, "workflow_chat", data.message, "".join(output))
            yield "event: done\ndata: done\n\n"
        return StreamingResponse(stream(), media_type="text/event-stream", headers={"Cache-Control": "no-cache"})

    @app.post("/api/features/{feature_id}/completed-chat")
    def completed_chat(feature_id: str, data: ChatIn, request: Request) -> StreamingResponse:
        """Explain an immutable completed record without restarting refinement."""
        try:
            solution = workflow.completed_solution(feature_id, request.state.locale)
            progress = workflow.solution_progress(feature_id, request.state.locale)
            base_url, api_key, model = provider_settings.credentials("completed_solution_chat")
            provider = OpenAICompatibleProvider(base_url, api_key, model)
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

        def stream():
            output: list[str] = []
            sent = 0
            evidence = {
                "completed_solution": solution,
                "work_log": [
                    {key: value for key, value in entry.items() if key != "image_data"}
                    for entry in progress["entries"]
                ],
                "checklist": progress["checklist"],
            }
            messages = [
                {
                    "role": "system",
                    "content": (
                        "Explain this completed Solution from its preserved record and evidence only. "
                        "The record is immutable: never propose applying refinement or changing its fields. "
                        "Clearly say when evidence was not recorded. If the user identifies new work, "
                        "recommend the explicit Create follow-up Problem action instead of reopening this Solution."
                    ),
                },
                {"role": "system", "content": response_language_instruction(request.state.locale)},
                {"role": "system", "content": json.dumps(evidence, ensure_ascii=False)},
                *workflow.chat_history("features", feature_id),
                {"role": "user", "content": data.message},
            ]
            try:
                for text in provider.stream(messages):
                    remaining = CHAT_RESPONSE_CHARACTER_LIMIT - sent
                    if remaining <= 0:
                        break
                    compact = text[:remaining]
                    sent += len(compact)
                    output.append(compact)
                    yield f"data: {compact.replace(chr(10), ' ')}\n\n"
            except (OSError, ValueError) as error:
                yield f"event: error\ndata: AI service is temporarily unavailable ({error}). Please try again.\n\n"
                return
            workflow.record_ai_run("features", feature_id, "workflow_chat", data.message, "".join(output))
            yield "event: done\ndata: done\n\n"

        return StreamingResponse(stream(), media_type="text/event-stream", headers={"Cache-Control": "no-cache"})

    @app.post("/api/{entity_type}/{entity_id}/draft")
    def draft(entity_type: str, entity_id: str, request: Request) -> dict[str, object]:
        """Generate a proposal only; the browser explicitly applies a reviewed draft."""
        try:
            context = workflow.context_for(entity_type, entity_id, request.state.locale)
            next_stage = {"captures": "problems", "problems": "features"}.get(entity_type)
            if not next_stage:
                raise WorkflowError("This workflow item has no next stage")
            base_url, api_key, model = provider_settings.credentials("problem_drafting" if next_stage == "problems" else "solution_drafting")
            provider = OpenAICompatibleProvider(base_url, api_key, model)
            result = provider.complete_json(
                [
                    {"role": "system", "content": bilingual_draft_prompt(entity_type, context["title"], context["detail"])},
                    *workflow.chat_history(entity_type, entity_id),
                ],
                f"{entity_type} draft",
            )
            if set(result) == set(SUPPORTED_LOCALES):
                versions = validate_bilingual_draft(entity_type, result)
                reviewed = versions[request.state.locale]
            else:
                # Older compatible providers can fail to honor the new outer
                # schema. Preserve their successful source-locale draft and
                # disclose the missing variant instead of losing user work.
                reviewed = validate_draft(entity_type, result)
                versions = {request.state.locale: reviewed}
            payload = {
                **reviewed,
                "source_locale": request.state.locale,
                "localized_versions": versions,
                "missing_locales": [locale for locale in SUPPORTED_LOCALES if locale not in versions],
            }
            workflow.record_ai_run(entity_type, entity_id, "workflow_draft", "Create a reviewed bilingual draft", str(payload))
            return payload
        except (WorkflowError, ValueError, OSError) as error:
            raise HTTPException(502, f"AI drafting failed: {error}") from error

    @app.post("/api/{entity_type}/{entity_id}/refine")
    def refine(entity_type: str, entity_id: str, request: Request) -> dict[str, object]:
        """Prepare a reviewed update for this item, without progressing its workflow state."""
        try:
            context = workflow.context_for(entity_type, entity_id, request.state.locale)
            task = {"captures": "capture_assistance", "problems": "problem_assistance", "features": "solution_assistance"}.get(entity_type)
            base_url, api_key, model = provider_settings.credentials(task)
            provider = OpenAICompatibleProvider(base_url, api_key, model)
            result = provider.complete_json(
                [
                    {"role": "system", "content": bilingual_refinement_prompt(entity_type, context["title"], context["detail"])},
                    *([{"role": "system", "content": response_language_instruction(request.state.locale)}] if entity_type == "captures" else []),
                    *workflow.chat_history(entity_type, entity_id),
                ],
                f"{entity_type} refinement",
            )
            if entity_type in {"problems", "features"} and set(result) == set(SUPPORTED_LOCALES):
                versions = {locale: validate_refinement(entity_type, dict(result[locale])) for locale in SUPPORTED_LOCALES}
                reviewed = versions[request.state.locale]
            else:
                reviewed = validate_refinement(entity_type, result)
                versions = {request.state.locale: reviewed} if entity_type in {"problems", "features"} else {}
            payload = {
                **reviewed,
                "source_note": context["detail"] or context["title"],
                "localized_versions": versions,
                "missing_locales": [locale for locale in SUPPORTED_LOCALES if locale not in versions] if versions else [],
            }
            workflow.record_ai_run(entity_type, entity_id, "workflow_refinement", "Refine current item", str(payload))
            return payload
        except (WorkflowError, ValueError, OSError) as error:
            raise HTTPException(502, f"AI refinement failed: {error}") from error

    @app.get("/api/{entity_type}/{entity_id}/refinement-context")
    def refinement_context(entity_type: str, entity_id: str, request: Request) -> dict[str, object]:
        try:
            return workflow.refinement_context_summary(entity_type, entity_id, locale=request.state.locale)
        except WorkflowError as error:
            status = 404 if str(error) == "Item not found" else 400
            raise HTTPException(status, str(error)) from error

    @app.post("/api/captures/{capture_id}/promote", status_code=201)
    def promote(capture_id: str, data: ProblemIn) -> dict[str, str]:
        try:
            return workflow.promote_capture(capture_id, data.statement, data.detail, data.localized_versions)
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/problems/{problem_id}/approve", status_code=204)
    def approve_problem(problem_id: str) -> None:
        try:
            workflow.approve_problem(problem_id)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/problems/{problem_id}/features", status_code=201)
    def feature(problem_id: str, data: FeatureIn) -> dict[str, str]:
        try:
            return workflow.create_feature(problem_id, data.title, data.outcome, data.non_goals, data.validation_criteria, data.localized_versions)
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

    @app.put("/api/features/{feature_id}/conflict")
    def conflict(feature_id: str, data: ConflictIn) -> dict[str, object]:
        try:
            addressed_report_id = data.address.conflict_report_id if data.address else ""
            if data.address and not addressed_report_id:
                prior = workflow.db.execute(
                    "SELECT id FROM conflict_reports WHERE feature_id=? AND state='conflicted' ORDER BY created_at DESC,rowid DESC LIMIT 1",
                    (feature_id,),
                ).fetchone()
                addressed_report_id = str(prior["id"]) if prior else ""
            report_id = workflow.record_conflict_evaluation(feature_id, data.state, data.citation, commit=False)
            address = None
            if data.address:
                if not addressed_report_id:
                    raise WorkflowError("No detected conflict is available to address")
                address = workflow.record_conflict_address(
                    feature_id,
                    addressed_report_id,
                    "addressed" if data.state == "clear" else "unaddressed",
                    data.address.basis,
                    data.address.disposition,
                    data.address.summary,
                    data.address.evidence_source_type,
                    data.address.evidence_source_id,
                    commit=False,
                )
            workflow.db.commit()
            return {"state": data.state, "report_id": report_id, "address": address}
        except WorkflowError as error:
            workflow.db.rollback()
            raise HTTPException(400, str(error)) from error

    @app.post("/api/features/{feature_id}/conflict-review", status_code=202)
    def conflict_review(feature_id: str, request: Request) -> dict[str, object]:
        """Prepare a cited AI review; it never changes the Solution or its conflict state."""
        try:
            board = workflow.board(request.state.locale)
            feature = next((item for item in board["features"] if item["id"] == feature_id), None)
            if not feature:
                raise WorkflowError("Solution not found")
            problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
            return conflict_reviews.start(feature, problem, response_language_instruction(request.state.locale))
        except (WorkflowError, ValueError, OSError) as error:
            raise HTTPException(502, f"Conflict review failed: {error}") from error

    @app.get("/api/conflict-reviews/{run_id}")
    def conflict_review_status(run_id: str) -> dict[str, object]:
        snapshot = conflict_reviews.get(run_id)
        if not snapshot:
            raise HTTPException(404, "Conflict review not found")
        return snapshot

    @app.delete("/api/conflict-reviews/{run_id}")
    def cancel_conflict_review(run_id: str) -> dict[str, object]:
        snapshot = conflict_reviews.cancel(run_id)
        if not snapshot:
            raise HTTPException(404, "Conflict review not found")
        workflow.cancel_conflict_review(run_id, snapshot)
        return snapshot

    @app.post("/api/features/{feature_id}/approve", status_code=204)
    def approve_feature(feature_id: str) -> None:
        try:
            workflow.approve_feature(feature_id)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.put("/api/features/{feature_id}/stage", status_code=204)
    def feature_stage(feature_id: str, data: FeatureStageIn) -> None:
        try:
            workflow.set_feature_stage(feature_id, data.state)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.get("/api/features/{feature_id}/progress")
    def solution_progress(feature_id: str, request: Request) -> dict[str, object]:
        try:
            return workflow.solution_progress(feature_id, request.state.locale)
        except WorkflowError as error:
            raise HTTPException(404, str(error)) from error

    @app.post("/api/features/{feature_id}/progress", status_code=201)
    def add_solution_progress(feature_id: str, data: SolutionProgressIn) -> dict[str, object]:
        try:
            return workflow.add_solution_progress(feature_id, data.body, data.image_data, data.image_media_type)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/progress/{entry_id}/summarize-image")
    def summarize_solution_image(entry_id: str, request: Request) -> dict[str, object]:
        """Use the configured image-summary task model; never hard-code a vision provider."""
        row = workflow.db.execute("SELECT image_data,image_media_type FROM solution_progress_entries WHERE id=?", (entry_id,)).fetchone()
        if not row:
            raise HTTPException(404, "Progress record not found")
        if not row[0]:
            raise HTTPException(400, "This progress record has no image")
        try:
            base_url, api_key, model = provider_settings.credentials("image_summary")
            content: list[object] = [
                {"type": "text", "text": "Return JSON only with exactly this shape: {\"ko\":{\"summary\":string},\"en\":{\"summary\":string}}. Summarize this work-progress image accurately and concisely in natural Korean and English in this one response. Both versions must describe the same visible evidence. Do not infer invisible details."},
                {"type": "image_url", "image_url": {"url": f"data:{row[1] or 'image/png'};base64,{row[0]}"}},
            ]
            result = OpenAICompatibleProvider(base_url, api_key, model).complete_json([{"role": "user", "content": content}], "progress image summary")
            versions = validate_bilingual_image_summary(result)
            workflow.set_solution_progress_summaries(entry_id, versions, request.state.locale)
            return {
                "summary": versions[request.state.locale]["image_summary"],
                "model": model,
                "localized_versions": versions,
                "missing_locales": [],
            }
        except (ValueError, OSError) as error:
            raise HTTPException(502, f"Image summary failed: {error}") from error

    @app.post("/api/progress/{entry_id}/comments", status_code=201)
    def add_solution_comment(entry_id: str, data: SolutionCommentIn) -> dict[str, object]:
        try:
            return workflow.add_solution_comment(entry_id, data.body)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/features/{feature_id}/checklist", status_code=201)
    def add_solution_checklist(feature_id: str, data: SolutionChecklistIn) -> dict[str, object]:
        try:
            return workflow.add_solution_checklist_item(feature_id, data.body)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.put("/api/checklist/{item_id}", status_code=204)
    def update_solution_checklist(item_id: str, data: SolutionChecklistIn) -> None:
        try:
            workflow.update_solution_checklist_item(item_id, data.body, data.checked)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/problems/{problem_id}/importance")
    def importance(problem_id: str, data: ImportanceIn) -> dict[str, object]:
        try:
            return workflow.assess_importance(problem_id, data.alignment, data.impact, data.urgency, data.leverage, data.evidence)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.get("/api/dashboard")
    def dashboard() -> dict[str, object]:
        return workflow.dashboard()

    @app.post("/api/goals", status_code=201)
    def goal(data: GoalIn) -> dict[str, str]:
        return workflow.create_goal(data.title, data.description)

    @app.post("/api/features/{feature_id}/completion", status_code=201)
    def completion(feature_id: str, data: CompletionIn) -> dict[str, str]:
        try:
            return workflow.record_completion(feature_id, data.evidence, data.report, data.no_update_reason)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/features/{feature_id}/verify", status_code=204)
    def verify_completion(feature_id: str) -> None:
        try:
            workflow.verify_completion(feature_id)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/features/{feature_id}/completion-review")
    def completion_review(feature_id: str, request: Request) -> dict[str, object]:
        """AI assesses acceptance evidence; it cannot complete anything itself."""
        try:
            board = workflow.board(request.state.locale)
            feature = next((item for item in board["features"] if item["id"] == feature_id), None)
            if not feature:
                raise WorkflowError("Solution not found")
            problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
            progress = workflow.solution_progress(feature_id, request.state.locale)
            # The image summary is the canonical textual evidence for AI review. Raw
            # base64 images can make this JSON request many megabytes and cause the
            # provider to reject it before inference (HTTP 400).
            progress_entries = [
                {key: value for key, value in entry.items() if key != "image_data"}
                for entry in progress["entries"]
            ]
            base_url, api_key, model = provider_settings.credentials("completion_review")
            report = OpenAICompatibleProvider(base_url, api_key, model).complete_json([
                {"role": "system", "content": "Return JSON only: {\"resolution\":\"complete|partial|insufficient_evidence\",\"executive_summary\":string,\"what_changed\":[string],\"criteria_review\":[{\"criterion\":string,\"status\":\"met|partial|not_evidenced\",\"evidence\":string}],\"remaining_checklist\":[string],\"decision_rationale\":string,\"problem_recommendation\":\"complete|keep_open\",\"capture_recommendation\":\"complete|keep_open\"}. Create a concise factual completion report from supplied evidence only. Assess every Validation Criteria bullet separately. Cite the exact work log, comment, checklist state, or image summary that supports it; use 'No recorded evidence' rather than inferring. Do not claim implementation details, completion, or changes not present in the evidence. Never change state. No implementation instructions."},
                {"role": "system", "content": response_language_instruction(request.state.locale)},
                {"role": "user", "content": json.dumps({"problem": problem, "solution": feature, "validation_criteria": feature.get("validation_criteria", ""), "progress_records": progress_entries, "checklist": progress["checklist"]}, ensure_ascii=False)},
            ], "completion review")
            review_id = workflow.save_completion_review(feature_id, report)
            return {"review_id": review_id, "problem_id": feature["problem_id"], "report": report}
        except (WorkflowError, ValueError, OSError) as error:
            raise HTTPException(502, f"Completion review failed: {error}") from error

    @app.post("/api/problems/{problem_id}/complete")
    def complete_problem(problem_id: str, data: ProblemCompletionIn | None = None) -> dict[str, object]:
        try:
            workflow.complete_problem(problem_id, data.reason if data else "", data.review_id if data else "")
            return write_completion_playbook(problem_id, refresh_lineage=True)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error
        except OSError as error:
            raise HTTPException(409, f"Could not write completed-work Playbook: {error}") from error

    def ensure_completion_document_unmodified(problem_id: str) -> sqlite3.Row | None:
        existing = retrieval.db.execute(
            "SELECT path,source_hash FROM completion_playbooks WHERE problem_id=?", (problem_id,)
        ).fetchone()
        if existing:
            try:
                if digest(vault.read_text(str(existing["path"]))) != existing["source_hash"]:
                    raise WorkflowError("Completed-work Playbook was modified externally; review it before regenerating")
            except FileNotFoundError:
                pass
        return existing

    def generate_feature_lineage(feature_id: str, force: bool) -> dict[str, object]:
        """Build the current deterministic Lineage and optionally enrich its interpretations."""
        lineage = workflow.create_lineage_snapshot(feature_id, force=force)
        try:
            base_url, api_key, model = provider_settings.credentials("lineage_inference")
            context = report_context([lineage])
            result = OpenAICompatibleProvider(base_url, api_key, model).complete_json([
                {"role": "system", "content": "Return JSON only: {\"claims\":[{\"claim_key\":string,\"text\":string,\"confidence\":\"high|medium|low\",\"evidence_ids\":[string]}]}. Add only useful likely rationale or relationship interpretations that are not already explicit. Every claim is AI inferred and must cite supplied evidence IDs. Never claim a Conflict is Addressed or Resolved, never create facts, and return an empty claims list when no inference is warranted."},
                {"role": "user", "content": context},
            ], "lineage inference")
            valid = validate_inference_payload(result, set(lineage["evidence"]))
            if valid:
                return workflow.add_lineage_inferences(feature_id, str(lineage["snapshot_id"]), valid)
            return workflow.mark_lineage_inference_complete(feature_id, str(lineage["snapshot_id"]))
        except (ValueError, OSError) as error:
            return workflow.set_lineage_inference_error(feature_id, str(lineage["snapshot_id"]), str(error))

    def current_problem_lineages(problem_id: str, refresh: bool) -> list[dict[str, object]]:
        feature_ids = [str(row[0]) for row in workflow.db.execute(
            "SELECT id FROM features WHERE problem_id=? ORDER BY created_at,rowid", (problem_id,)
        ).fetchall()]
        lineages: list[dict[str, object]] = []
        for feature_id in feature_ids:
            if refresh:
                lineages.append(generate_feature_lineage(feature_id, force=True))
                continue
            try:
                lineages.append(workflow.lineage(feature_id))
            except WorkflowError:
                lineages.append(generate_feature_lineage(feature_id, force=False))
        return lineages

    def write_completion_playbook(problem_id: str, refresh_lineage: bool = False) -> dict[str, object]:
        """Regenerate a human-approved completion projection from validated Lineage."""
        existing = ensure_completion_document_unmodified(problem_id)
        directory = str(existing["path"]).rsplit("/", 1)[0] if existing else f"{date.today().year}/90. Archive/Completed Work"
        lineages = current_problem_lineages(problem_id, refresh_lineage)
        lineage_context = readable_report_context(lineages)
        report_input_hash = digest(lineage_context)
        raw_path, raw_content = workflow.completion_playbook(problem_id, directory, raw=True, lineages=lineages)
        executive_summary = ""
        report_body = ""
        report_generation_status = "deterministic_fallback"
        try:
            base_url, api_key, model = provider_settings.credentials("completion_report")
            report = OpenAICompatibleProvider(base_url, api_key, model).complete_json([
                {"role": "system", "content": "You are an exceptional CTO preparing English-canonical portable Knowledge from a validated Lineage Knowledge projection. Return JSON only: {\"executive_summary_markdown\":string,\"report_body_markdown\":string}. Write every heading and all prose in natural English while preserving code, quoted evidence, and the Observed, Decided, and Inferred distinctions. executive_summary_markdown is a retrieval index, not prose: use only compact Markdown bullets under exactly these headings: ### Completed, ### Decisions, ### Verification, ### Risks and follow-up. report_body_markdown must use exactly: ### Why, ### What changed, ### How the work was carried out, ### Final verification, ### Decision and risks. Use only claims and referenced_evidence in the supplied Lineage snapshots; cite only their human-readable evidence labels. Never output UUIDs, database IDs, snapshot IDs, claim IDs, revision IDs, or source IDs. Label AI inference and uncertainty. Say 'Not explicitly recorded' or 'No recorded evidence' instead of inventing facts. Do not use the report itself as historical evidence. Keep the combined output under 650 words."},
                {"role": "user", "content": lineage_context},
            ], "completion executive summary")
            executive_summary = str(report.get("executive_summary_markdown", "")).strip()
            report_body = str(report.get("report_body_markdown", "")).strip()
            if executive_summary or report_body:
                report_generation_status = "generated"
                workflow.record_ai_run("problems", problem_id, "completion_executive_summary", f"Lineage snapshots: {','.join(str(item['snapshot_id']) for item in lineages)}", json.dumps({"executive_summary": executive_summary, "report_body": report_body}, ensure_ascii=False))
        except (ValueError, OSError):
            # The archive still preserves the facts when the optional provider
            # is offline; it is never replaced with invented prose.
            executive_summary = ""
        path, content = workflow.completion_playbook(
            problem_id,
            directory,
            executive_summary=executive_summary,
            report_body=report_body,
            lineages=lineages,
        )
        if existing:
            old_path = str(existing["path"])
            if old_path != path:
                if (vault.root / path).exists():
                    raise WorkflowError("A completed-work document already uses this human-readable name")
                vault.move(old_path, path)
                old_raw = old_path.rsplit("/", 1)[0] + "/Raw/" + old_path.rsplit("/", 1)[1][:-3] + ".raw.md"
                if (vault.root / old_raw).exists() and not (vault.root / raw_path).exists():
                    vault.move(old_raw, raw_path)
            legacy_raw = old_path.rsplit("/", 1)[0] + "/Raw/" + old_path.rsplit("/", 1)[1][:-3] + ".raw.md"
            if (vault.root / legacy_raw).exists() and not (vault.root / raw_path).exists():
                vault.move(legacy_raw, raw_path)
        vault.atomic_write(path, content)
        knowledge_cache.invalidate(path)
        # Regeneration is an explicit human action. Raw Data is regenerated from
        # the immutable workflow record alongside the concise main document.
        vault.atomic_write(raw_path, raw_content)
        for asset_path, asset_content in workflow.completion_assets(problem_id, directory):
            vault.atomic_write_bytes(asset_path, asset_content)
        selected = lineages[-1] if lineages else None
        workflow.remember_completion_playbook(
            problem_id,
            path,
            digest(content),
            str(selected["snapshot_id"]) if selected else "",
            int(selected["version"]) if selected else 0,
            report_input_hash,
            report_generation_status,
        )
        retrieval.index_changed()
        return {
            "path": path,
            "raw_path": raw_path,
            "lineage": {
                "snapshot_id": selected["snapshot_id"],
                "status": selected["status"],
                "version": selected["version"],
                "retryable": bool(selected.get("generation", {}).get("inference_error")),
            } if selected else None,
            "report_generation": {
                "status": report_generation_status,
                "lineage_snapshot_id": selected["snapshot_id"] if selected else "",
                "lineage_version": selected["version"] if selected else 0,
            },
        }

    @app.post("/api/problems/{problem_id}/completion-playbook/regenerate")
    def regenerate_completion_playbook(problem_id: str) -> dict[str, object]:
        problem = workflow.db.execute("SELECT state FROM problems WHERE id=?", (problem_id,)).fetchone()
        if not problem or problem["state"] != "completed":
            raise HTTPException(400, "Only a completed Problem can regenerate its completed-work document")
        try:
            return write_completion_playbook(problem_id, refresh_lineage=True)
        except WorkflowError as error:
            raise HTTPException(409, str(error)) from error
        except OSError as error:
            raise HTTPException(409, f"Could not regenerate completed-work Playbook: {error}") from error

    @app.get("/api/features/{feature_id}/lineage")
    def feature_lineage(feature_id: str) -> dict[str, object]:
        try:
            return workflow.lineage(feature_id)
        except WorkflowError as error:
            raise HTTPException(404, str(error)) from error

    @app.get("/api/features/{feature_id}/lineage/evidence/{evidence_id}")
    def feature_lineage_evidence(feature_id: str, evidence_id: str) -> dict[str, object]:
        try:
            return workflow.lineage_evidence(feature_id, evidence_id)
        except WorkflowError as error:
            raise HTTPException(404, str(error)) from error

    @app.post("/api/features/{feature_id}/lineage/regenerate", status_code=201)
    def regenerate_feature_lineage(feature_id: str, data: LineageRegenerateIn) -> dict[str, object]:
        feature = workflow.db.execute(
            """SELECT f.problem_id,p.state AS problem_state,c.state AS completion_state
               FROM features f JOIN problems p ON p.id=f.problem_id
               LEFT JOIN completions c ON c.feature_id=f.id WHERE f.id=?""",
            (feature_id,),
        ).fetchone()
        if not feature or (feature["problem_state"] != "completed" and feature["completion_state"] != "verified"):
            raise HTTPException(400, "Only a completed Solution can regenerate Lineage")
        try:
            ensure_completion_document_unmodified(str(feature["problem_id"]))
            lineage = generate_feature_lineage(feature_id, force=True) if data.include_inference else workflow.create_lineage_snapshot(feature_id, force=True)
            projection = write_completion_playbook(str(feature["problem_id"]))
            return {**lineage, "document_sync": projection}
        except WorkflowError as error:
            raise HTTPException(409, str(error)) from error
        except OSError as error:
            raise HTTPException(409, f"Could not synchronize completed-work Playbook: {error}") from error

    @app.post("/api/features/{feature_id}/lineage/claims/{claim_id}/corrections", status_code=201)
    def correct_feature_lineage_claim(feature_id: str, claim_id: str, data: LineageCorrectionIn) -> dict[str, object]:
        try:
            revision = workflow.correct_lineage_claim(
                feature_id,
                claim_id,
                data.text,
                data.reason,
                data.current_revision_id,
            )
            feature = workflow.db.execute("SELECT problem_id FROM features WHERE id=?", (feature_id,)).fetchone()
            document_sync: dict[str, object]
            try:
                document_sync = {"status": "updated", **write_completion_playbook(str(feature["problem_id"]))}
            except (WorkflowError, OSError) as error:
                document_sync = {"status": "needs_retry", "error": str(error)}
            return {**revision, "document_sync": document_sync}
        except WorkflowError as error:
            status = 409 if "reload" in str(error).lower() else 400
            raise HTTPException(status, str(error)) from error

    @app.delete("/api/problems/{problem_id}/completion-playbook", status_code=204)
    def delete_completion_playbook(problem_id: str, force: bool = False) -> None:
        row = workflow.db.execute("SELECT path,source_hash FROM completion_playbooks WHERE problem_id=?", (problem_id,)).fetchone()
        if not row:
            raise HTTPException(404, "Completed-work document is not tracked for this Problem")
        path = str(row["path"])
        externally_modified = False
        try:
            externally_modified = digest(vault.read_text(path)) != str(row["source_hash"])
        except FileNotFoundError:
            pass
        if externally_modified and not force:
            raise HTTPException(409, "This completed-work document was modified outside LLM Wiki. Delete anyway to remove the document, Raw Data, and generated captures.")
        directory, filename = path.rsplit("/", 1)
        raw_path = f"{directory}/assets/{filename[:-3]}.raw.md"
        legacy_raw_path = f"{directory}/Raw/{filename[:-3]}.raw.md"
        vault.remove(path)
        knowledge_cache.invalidate(path)
        vault.remove(raw_path)
        vault.remove(legacy_raw_path)
        for asset_path, _ in workflow.completion_assets(problem_id, directory):
            vault.remove(asset_path)
        workflow.forget_completion_playbook(problem_id)
        retrieval.index_changed()

    @app.get("/api/features/{feature_id}/handoff", response_class=PlainTextResponse)
    def handoff(feature_id: str) -> str:
        try:
            return workflow.handoff(feature_id)
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/features/{feature_id}/patches", status_code=201)
    def patch_proposal(feature_id: str, data: PatchIn, request: Request) -> dict[str, str]:
        try:
            before = vault.read_text(data.path)
            content = data.content
            managed = "llm_wiki_managed: true" in before and "canonical_locale: en" in before
            if managed and request.state.locale == "ko":
                base_url, api_key, model = provider_settings.credentials("knowledge_translation")
                normalized = OpenAICompatibleProvider(base_url, api_key, model).complete_json(
                    [
                        {
                            "role": "system",
                            "content": (
                                'Return JSON only: {"content":string}. Translate the supplied reviewed Knowledge patch '
                                "into natural English for an English-canonical Markdown document. Preserve Markdown structure, "
                                "code, identifiers, citations, quoted evidence, URLs, and wiki-link targets exactly. Never add facts."
                            ),
                        },
                        {"role": "user", "content": content},
                    ],
                    "knowledge English normalization",
                )
                content = str(normalized.get("content", "")).strip()
                if not content:
                    raise ValueError("Knowledge English normalization was empty")
            patch = propose_section_patch(before, data.operation, data.heading, content)
            return workflow.save_patch_proposal(feature_id, data.path, data.operation, data.heading, content, patch.base_hash, patch.before, patch.proposed)
        except (WorkflowError, ValueError, OSError) as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/patches/{patch_id}/apply", status_code=204)
    def apply_patch(patch_id: str) -> None:
        try:
            stored = workflow.patch(patch_id)
            patch = SectionPatch(stored["operation"], stored["heading"], stored["content"], stored["base_hash"], stored["before_text"], stored["proposed_text"])
            apply_reviewed_patch(vault, stored["path"], patch)
            knowledge_cache.invalidate(stored["path"])
            workflow.mark_patch_applied(patch_id)
            retrieval.index_changed()
        except (WorkflowError, PatchConflict, OSError) as error:
            raise HTTPException(409, str(error)) from error

    @app.post("/api/patches/{patch_id}/undo", status_code=204)
    def undo_patch(patch_id: str) -> None:
        try:
            stored = workflow.undo_patch(patch_id)
            vault.atomic_write(stored["path"], stored["reverse_text"])
            knowledge_cache.invalidate(stored["path"])
            retrieval.index_changed()
        except (WorkflowError, OSError) as error:
            raise HTTPException(409, str(error)) from error

    @app.get("/api/provider/config")
    def provider_config() -> dict[str, object]:
        return provider_settings.public()

    @app.put("/api/provider/config")
    def save_provider_config(data: ProviderConfigIn) -> dict[str, object]:
        try:
            provider_settings.save(data.base_url, data.model, data.api_key, data.advanced_model, data.advanced_tasks, data.report_language)
            return provider_settings.public()
        except Exception as error:
            raise HTTPException(400, f"Could not save provider configuration: {error}") from error

    @app.post("/api/provider/test")
    def provider_health() -> dict[str, object]:
        try:
            base_url, api_key, model = provider_settings.credentials("problem_enrichment")
            models = OpenAICompatibleProvider(base_url, api_key, model).models()
            return {"models": models, "configured_model": model}
        except Exception as error:  # provider failures must not affect local operation
            raise HTTPException(502, f"Provider health check failed: {error}") from error

    @app.post("/api/ai/enrich-problem")
    def enrich_problem(data: EnrichIn, request: Request) -> dict[str, object]:
        try:
            base_url, api_key, model = provider_settings.credentials()
            provider = OpenAICompatibleProvider(base_url, api_key, model)
            statement = data.statement + "\n\n" + response_language_instruction(request.state.locale)
            return enrich_problem_graph(AIEnrichmentEngine(provider), statement, data.citations[:8])
        except (ValueError, OSError) as error:
            raise HTTPException(502, str(error)) from error

    @app.get("/api/knowledge")
    def read_knowledge(
        request: Request,
        path: str = Query(min_length=1),
        locale: str | None = None,
        translate: bool = True,
    ) -> dict[str, object]:
        """Read canonical Markdown or a hash-current derived Korean view."""
        requested = normalize_locale(locale, request.state.locale) if locale else request.state.locale
        try:
            canonical = vault.read_text(path)
        except (FileNotFoundError, ValueError) as error:
            raise HTTPException(404, "Knowledge document not found") from error
        source_hash = digest(canonical)
        managed = "llm_wiki_managed: true" in canonical and "canonical_locale: en" in canonical
        base = {
            "path": path,
            "canonical_locale": "en" if managed else "original",
            "served_locale": "en" if managed else "original",
            "translated": False,
            "cache_status": "not_applicable",
            "source_hash": source_hash,
        }
        if requested != "ko" or not managed:
            return {**base, "markdown": canonical}
        cached = knowledge_cache.get(path, "ko", source_hash)
        if cached:
            return {**base, "markdown": cached["translated_markdown"], "served_locale": "ko", "translated": True, "cache_status": "hit"}
        if not translate:
            return {**base, "markdown": canonical, "cache_status": "pending"}
        try:
            base_url, api_key, model = provider_settings.credentials("knowledge_translation")
            result = OpenAICompatibleProvider(base_url, api_key, model).complete_json(
                [
                    {
                        "role": "system",
                        "content": (
                            "Return JSON only: {\"markdown\":string}. Translate the readable prose of this English canonical Markdown "
                            "into natural Korean. Preserve frontmatter keys and values, heading levels, code fences and code, identifiers, "
                            "citations, quoted evidence, URLs, and Markdown or wiki-link targets exactly. Never add facts."
                        ),
                    },
                    {"role": "user", "content": canonical},
                ],
                "knowledge Korean translation",
            )
            translated = str(result.get("markdown", "")).strip()
            if not translated:
                raise ValueError("Knowledge translation was empty")
            current_hash = digest(vault.read_text(path))
            if not knowledge_cache.put(path, "ko", source_hash, translated, model, current_source_hash=current_hash):
                return {**base, "markdown": canonical, "cache_status": "fallback", "warning_code": "canonical_changed"}
            return {**base, "markdown": translated, "served_locale": "ko", "translated": True, "cache_status": "miss"}
        except (ValueError, OSError):
            return {**base, "markdown": canonical, "cache_status": "fallback", "warning_code": "translation_unavailable"}

    @app.post("/api/knowledge/translation-cancel", status_code=204)
    def stop_knowledge_translation(request_id: str = Query(min_length=1, max_length=200)) -> Response:
        cancel_knowledge_translation(request_id)
        return Response(status_code=204)

    @app.get("/api/knowledge/translate")
    def translate_knowledge_progressively(
        path: str = Query(min_length=1),
        request_id: str = Query(min_length=1, max_length=200),
    ) -> StreamingResponse:
        try:
            canonical = vault.read_text(path)
        except (FileNotFoundError, ValueError) as error:
            raise HTTPException(404, "Knowledge document not found") from error
        if "llm_wiki_managed: true" not in canonical or "canonical_locale: en" not in canonical:
            raise HTTPException(409, "Knowledge document is not managed English canonical content")
        source_hash = digest(canonical)
        blocks = knowledge_translation_blocks(canonical)
        translatable = [index for index, block in enumerate(blocks) if block["translatable"]]

        def events():
            cancel_event = threading.Event()
            with knowledge_translation_lock:
                knowledge_translation_jobs[request_id] = cancel_event
                if request_id in knowledge_translation_cancelled_ids:
                    cancel_event.set()
            try:
                cached = knowledge_cache.get(path, "ko", source_hash)
                if cached:
                    yield json.dumps(
                        {"event": "complete", "markdown": cached["translated_markdown"], "cache_status": "hit"},
                        ensure_ascii=False,
                    ) + "\n"
                    return
                base_url, api_key, model = provider_settings.credentials("knowledge_translation")
                provider = OpenAICompatibleProvider(base_url, api_key, model)
                completed = 0
                for index in translatable:
                    if cancel_event.is_set():
                        yield json.dumps({"event": "cancelled"}) + "\n"
                        return
                    original = str(blocks[index]["markdown"])
                    source = original.strip()
                    result = provider.complete_json(
                        [
                            {
                                "role": "system",
                                "content": (
                                    "Return JSON only: {\"markdown\":string}. Translate this complete English Markdown paragraph "
                                    "into natural Korean. Preserve heading levels, code, identifiers, citations, quoted evidence, "
                                    "URLs, and Markdown or wiki-link targets exactly. Never add facts."
                                ),
                            },
                            {"role": "user", "content": source},
                        ],
                        "knowledge Korean paragraph translation",
                    )
                    translated = str(result.get("markdown", "")).strip()
                    if not translated:
                        raise ValueError("Knowledge paragraph translation was empty")
                    trailing = original[len(original.rstrip()):]
                    blocks[index]["markdown"] = translated + trailing
                    completed += 1
                    yield json.dumps(
                        {
                            "event": "paragraph",
                            "index": index,
                            "markdown": translated,
                            "completed": completed,
                            "total": len(translatable),
                        },
                        ensure_ascii=False,
                    ) + "\n"
                if cancel_event.is_set():
                    yield json.dumps({"event": "cancelled"}) + "\n"
                    return
                translated_markdown = "".join(
                    str(block["prefix"]) + str(block["markdown"]) for block in blocks
                )
                current_hash = digest(vault.read_text(path))
                if not knowledge_cache.put(
                    path,
                    "ko",
                    source_hash,
                    translated_markdown,
                    model,
                    current_source_hash=current_hash,
                ):
                    yield json.dumps({"event": "error", "warning_code": "canonical_changed"}) + "\n"
                    return
                yield json.dumps(
                    {"event": "complete", "markdown": translated_markdown, "cache_status": "miss"},
                    ensure_ascii=False,
                ) + "\n"
            except (ValueError, OSError):
                yield json.dumps({"event": "error", "warning_code": "translation_unavailable"}) + "\n"
            finally:
                with knowledge_translation_lock:
                    knowledge_translation_jobs.pop(request_id, None)
                    knowledge_translation_cancelled_ids.discard(request_id)

        return StreamingResponse(events(), media_type="application/x-ndjson")

    @app.post("/api/{entity_type}/{entity_id}/project", status_code=201)
    def project(entity_type: str, entity_id: str) -> dict[str, str]:
        try:
            path, content = workflow.projection(entity_type, entity_id)
            try:
                existing = vault.read_text(path)
                previous = retrieval.db.execute("SELECT source_hash FROM mirror_files WHERE entity_type=? AND entity_id=?", (entity_type, entity_id)).fetchone()
                if not previous or digest(existing) != previous[0]:
                    raise WorkflowError("Generated file was modified externally; import or regenerate after review")
            except FileNotFoundError:
                pass
            vault.atomic_write(path, content)
            knowledge_cache.invalidate(path)
            workflow.mirror(entity_type, entity_id, path, digest(content))
            retrieval.index_changed()
            return {"path": path}
        except (WorkflowError, OSError) as error:
            raise HTTPException(409, str(error)) from error

    @app.post("/api/{entity_type}/{entity_id}/archive", status_code=204)
    def archive(entity_type: str, entity_id: str) -> None:
        try:
            source = workflow.archive(entity_type, entity_id)
            destination = f"{date.today().year}/90. Archive/{entity_type.title()}/{Path(source).name}"
            vault.move(source, destination)
            workflow.mirror(entity_type, entity_id, destination, digest(vault.read_text(destination)))
            retrieval.index_changed()
        except (WorkflowError, OSError) as error:
            raise HTTPException(409, str(error)) from error

    @app.get("/")
    def shell() -> FileResponse:
        # The local single-file shell changes frequently while the workbench evolves;
        # do not leave a long-lived browser tab on an obsolete UI bundle.
        return FileResponse(Path(__file__).parent.parent / "static" / "index.html", headers={"Cache-Control": "no-store"})

    return app
