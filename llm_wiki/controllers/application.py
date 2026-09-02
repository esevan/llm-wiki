"""HTTP routes bound to an application runtime."""

from __future__ import annotations

import asyncio
import json
import sqlite3
from contextlib import asynccontextmanager
from datetime import date
from pathlib import Path

from fastapi import FastAPI, HTTPException, Query, Request
from fastapi.responses import (
    FileResponse,
    JSONResponse,
    PlainTextResponse,
    Response,
    StreamingResponse,
)

from llm_wiki.controllers.jobs import job_view
from llm_wiki.controllers.jobs import router as jobs_router
from llm_wiki.controllers.schemas import (
    CaptureIn,
    ChatIn,
    CompletionIn,
    ConflictIn,
    ConflictResolutionsIn,
    EnrichIn,
    FeatureIn,
    FeatureStageIn,
    GoalIn,
    ImportanceIn,
    LineageCorrectionIn,
    LineageRegenerateIn,
    LocaleIn,
    LocalizedSupplementIn,
    ManualUpdateIn,
    PatchIn,
    ProblemCompletionIn,
    ProblemIn,
    ProviderConfigIn,
    SolutionChecklistIn,
    SolutionCommentIn,
    SolutionProgressIn,
    TransitionIn,
    WorkbenchCategoryIn,
    WorkbenchImportanceIn,
)
from llm_wiki.core.jobs import TaskDescriptor
from llm_wiki.services.conversation import refinement_focus_prompt, system_prompt
from llm_wiki.services.handlers.conflict_review import conflict_review_query, conflict_source_hash
from llm_wiki.services.handlers.lineage import lineage_source_hash
from llm_wiki.services.handlers.organization import organization_items
from llm_wiki.services.localization import (
    SUPPORTED_LOCALES,
    load_locale_resources,
    localize_descriptor,
    normalize_locale,
    response_language_instruction,
)
from llm_wiki.services.patches import (
    PatchConflict,
    SectionPatch,
    apply_reviewed_patch,
    digest,
    propose_section_patch,
)
from llm_wiki.services.runtime import ApplicationRuntime
from llm_wiki.services.workflow import TRANSITIONS, WorkflowError, available_transitions

CHAT_RESPONSE_CHARACTER_LIMIT = 1_200


def create_http_app(runtime: ApplicationRuntime) -> FastAPI:
    vault = runtime.vault
    retrieval = runtime.retrieval
    workflow = runtime.workflow
    provider_settings = runtime.provider_settings
    locale_settings = runtime.locale_settings
    knowledge_cache = runtime.knowledge_cache
    completion_archive = runtime.completion_archive
    job_repository = runtime.job_repository
    fast_queue = runtime.fast_queue
    job_submission = runtime.job_submission

    async def enqueue(
        descriptor: TaskDescriptor,
        payload: dict[str, object],
        *,
        idempotency_key: str = "",
        source_hash: str = "",
        model_task: str | None = None,
    ) -> JSONResponse:
        job = await job_submission.enqueue(
            descriptor,
            payload,
            idempotency_key=idempotency_key,
            source_hash=source_hash,
            model_task=model_task,
        )
        return JSONResponse(job_view(job), status_code=202)

    enqueue_derived = job_submission.enqueue_derived_translation
    enqueue_embeddings = job_submission.enqueue_embeddings
    enqueue_completion_report = job_submission.enqueue_completion_report

    async def watch_vault() -> None:
        from watchfiles import (
            awatch,  # keep file-watching out of local request hot paths
        )

        async for _changes in awatch(vault.root):
            knowledge_cache.cleanup()
            await asyncio.to_thread(retrieval.index_changed)
            await enqueue_embeddings()
            async with app.state.index_condition:
                app.state.index_revision += 1
                app.state.index_condition.notify_all()

    @asynccontextmanager
    async def lifespan(app: FastAPI):
        # Structural indexing is intentionally separate from app construction and AI-free.
        await job_repository.initialize()
        knowledge_cache.cleanup()
        retrieval.index_changed()
        await enqueue_embeddings()
        watcher = asyncio.create_task(watch_vault())
        yield
        watcher.cancel()
        try:
            await watcher
        except asyncio.CancelledError:
            pass
        retrieval.db.close()

    app = FastAPI(title="LLM Wiki", lifespan=lifespan)
    app.include_router(jobs_router)
    app.state.retrieval = retrieval
    app.state.vault = vault
    app.state.workflow = workflow
    app.state.index_revision = 0
    app.state.index_condition = asyncio.Condition()
    app.state.provider_settings = provider_settings
    app.state.locale_settings = locale_settings
    app.state.knowledge_cache = knowledge_cache
    app.state.job_repository = job_repository
    app.state.fast_queue = fast_queue

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
    async def index() -> dict[str, int | float]:
        result = retrieval.index_changed()
        await enqueue_embeddings()
        return result

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
    async def search(
        q: str = Query(min_length=1, max_length=500),
        limit: int = Query(20, ge=1, le=100),
        offset: int = Query(0, ge=0),
        semantic: bool = False,
    ) -> dict[str, object]:
        if semantic and retrieval.status()["semantic_ready"] < retrieval.status()["documents"]:
            await enqueue_embeddings()
        return {
            "query": q,
            "offset": offset,
            "results": [result.__dict__ for result in retrieval.search(q, limit, semantic, offset)],
        }

    @app.post("/api/captures", status_code=201)
    async def capture(data: CaptureIn, request: Request) -> dict[str, str]:
        capture_id = workflow.capture(data.text)
        await enqueue_derived("captures", capture_id, "text", data.text, request.state.locale)
        return {"id": capture_id, "text": data.text}

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
    def list_transitions_for_item(
        entity_type: str, entity_id: str, request: Request
    ) -> dict[str, list[dict[str, object]]]:
        """Return transitions available for a specific item, filtered by its current state."""
        try:
            entity = None
            if entity_type in {"captures", "problems", "features"}:
                row = workflow.db.execute(f"SELECT * FROM {entity_type} WHERE id=?", (entity_id,)).fetchone()
                if row:
                    entity = dict(row)
            if entity is None:
                raise WorkflowError("Item not found")
            return {
                "transitions": localize_descriptor(available_transitions(entity_type, entity), request.state.locale)
            }
        except WorkflowError as error:
            raise HTTPException(404, str(error)) from error

    @app.post("/api/transitions/{entity_type}/{entity_id}", status_code=200)
    async def apply_transition(entity_type: str, entity_id: str, data: TransitionIn) -> dict[str, object]:
        """Apply a menu-only Workflow Transition with its required input form.

        This is the single entry point for all transitions, including the
        skip-conflict-check and complete-without-report paths.
        """
        try:
            result = workflow.apply_transition(data.transition_id, entity_type, entity_id, data.fields)
            # The solution_to_completed transition completes the Problem and writes
            # the completion playbook, so the frontend gets the archive path back.
            if (
                data.transition_id == "solution_to_completed"
                and "problem_id" in result
                and not result.get("note_skipped")
            ):
                try:
                    playbook = write_completion_playbook(str(result["problem_id"]), refresh_lineage=True)
                    result["playbook"] = playbook
                    result["report_job_id"] = await enqueue_completion_report(
                        str(result["problem_id"]), refresh_lineage=False
                    )
                except (WorkflowError, OSError) as error:
                    # The Problem is already completed; the playbook failure is non-fatal.
                    result["playbook_error"] = str(error)
            return result
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error
        except OSError as error:
            raise HTTPException(409, str(error)) from error

    @app.post("/api/workbench/organize", status_code=202)
    async def organize_workbench(request: Request) -> JSONResponse:
        items = organization_items(workflow, request.state.locale)
        source_hash = digest(json.dumps(items, sort_keys=True, ensure_ascii=False))
        return await enqueue(
            TaskDescriptor("workbench_organization", "workbench", "current", "workbench"),
            {"locale": request.state.locale},
            idempotency_key=f"workbench-organization:{source_hash}",
            source_hash=source_hash,
            model_task="workbench_organization",
        )

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
    def completed_solutions(
        request: Request, limit: int = Query(default=5, ge=1, le=50)
    ) -> dict[str, list[dict[str, object]]]:
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
            row = (
                workflow.db.execute(f"SELECT 1 FROM {entity_type} WHERE id=?", (entity_id,)).fetchone()
                if entity_type in {"problems", "features"}
                else None
            )
            if not row:
                raise WorkflowError("Item not found")
            workflow.localized.supplement(entity_type, entity_id, data.locale, data.fields)
            workflow.db.commit()
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/{entity_type}/{entity_id}/chat")
    async def chat(entity_type: str, entity_id: str, data: ChatIn, request: Request) -> StreamingResponse:
        try:
            context = workflow.context_for(entity_type, entity_id, request.state.locale)
            task = {
                "captures": "capture_assistance",
                "problems": "problem_assistance",
                "features": "solution_assistance",
            }.get(entity_type)
            base_url, api_key, model = provider_settings.credentials(task)
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

        async def stream():
            output: list[str] = []
            sent = 0
            assessment = (
                workflow.refinement_structure_assessment(entity_type, entity_id)
                if entity_type in {"problems", "features"}
                else {}
            )
            messages = [
                {"role": "system", "content": system_prompt(entity_type)},
                {"role": "system", "content": response_language_instruction(request.state.locale)},
                {
                    "role": "system",
                    "content": f"Current {context['type']}: {context['title']}\nKnown detail: {context['detail']}",
                },
                *([{"role": "system", "content": refinement_focus_prompt(assessment)}] if assessment else []),
                *workflow.chat_history(entity_type, entity_id),
                {"role": "user", "content": data.message},
            ]
            try:
                async for text in fast_queue.stream(base_url=base_url, api_key=api_key, model=model, messages=messages):
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
    async def next_chat(entity_type: str, entity_id: str, data: ChatIn, request: Request) -> StreamingResponse:
        """Collect the required information for the next stage before drafting it."""
        next_stage = {"captures": "problems", "problems": "features"}.get(entity_type)
        if not next_stage:
            raise HTTPException(400, "Solutions do not have a next workflow stage")
        try:
            context = workflow.context_for(entity_type, entity_id, request.state.locale)
            base_url, api_key, model = provider_settings.credentials(
                "problem_drafting" if next_stage == "problems" else "solution_drafting"
            )
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

        async def stream():
            output: list[str] = []
            sent = 0
            messages = [
                {"role": "system", "content": system_prompt(next_stage)},
                {"role": "system", "content": response_language_instruction(request.state.locale)},
                {
                    "role": "system",
                    "content": f"Source {context['type']}: {context['title']}\nKnown detail: {context['detail']}",
                },
                *workflow.chat_history(entity_type, entity_id),
                {"role": "user", "content": data.message},
            ]
            try:
                async for text in fast_queue.stream(base_url=base_url, api_key=api_key, model=model, messages=messages):
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
    async def completed_chat(feature_id: str, data: ChatIn, request: Request) -> StreamingResponse:
        """Explain an immutable completed record without restarting refinement."""
        try:
            solution = workflow.completed_solution(feature_id, request.state.locale)
            progress = workflow.solution_progress(feature_id, request.state.locale)
            base_url, api_key, model = provider_settings.credentials("completed_solution_chat")
        except (WorkflowError, ValueError) as error:
            raise HTTPException(400, str(error)) from error

        async def stream():
            output: list[str] = []
            sent = 0
            evidence = {
                "completed_solution": solution,
                "work_log": [
                    {key: value for key, value in entry.items() if key != "image_data"} for entry in progress["entries"]
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
                async for text in fast_queue.stream(base_url=base_url, api_key=api_key, model=model, messages=messages):
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

    @app.post("/api/{entity_type}/{entity_id}/draft", status_code=202)
    async def draft(entity_type: str, entity_id: str, request: Request) -> JSONResponse:
        """Queue an unapplied proposal bound to the originating surface."""
        try:
            item = workflow.context_for(entity_type, entity_id, request.state.locale)
            if entity_type not in {"captures", "problems"}:
                raise WorkflowError("This workflow item has no next stage")
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error
        source_hash = digest(f"{item['title']}\n{item['detail']}")
        surface_id = request.headers.get("X-LLM-Wiki-Surface", "")
        return await enqueue(
            TaskDescriptor("workflow_draft", entity_type, entity_id, "inline_preview"),
            {"locale": request.state.locale, "surface_id": surface_id},
            idempotency_key=f"draft:{entity_type}:{entity_id}:{source_hash}:{surface_id}",
            source_hash=source_hash,
            model_task="problem_drafting" if entity_type == "captures" else "solution_drafting",
        )

    @app.post("/api/{entity_type}/{entity_id}/refine", status_code=202)
    async def refine(entity_type: str, entity_id: str, request: Request) -> JSONResponse:
        """Queue an unapplied refinement bound to the originating surface."""
        try:
            item = workflow.context_for(entity_type, entity_id, request.state.locale)
            if entity_type not in {"captures", "problems", "features"}:
                raise WorkflowError("Unknown workflow refinement")
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error
        source_hash = digest(f"{item['title']}\n{item['detail']}")
        surface_id = request.headers.get("X-LLM-Wiki-Surface", "")
        return await enqueue(
            TaskDescriptor("workflow_refinement", entity_type, entity_id, "inline_preview"),
            {"locale": request.state.locale, "surface_id": surface_id},
            idempotency_key=f"refine:{entity_type}:{entity_id}:{source_hash}:{surface_id}",
            source_hash=source_hash,
            model_task={
                "captures": "capture_assistance",
                "problems": "problem_assistance",
                "features": "solution_assistance",
            }[entity_type],
        )

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
            return workflow.create_feature(
                problem_id, data.title, data.outcome, data.non_goals, data.validation_criteria, data.localized_versions
            )
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
    async def conflict_review(feature_id: str, request: Request) -> JSONResponse:
        """Prepare a cited AI review; it never changes the Solution or its conflict state."""
        try:
            board = workflow.board(request.state.locale)
            feature = next((item for item in board["features"] if item["id"] == feature_id), None)
            if not feature:
                raise WorkflowError("Solution not found")
            source_hash = conflict_source_hash(workflow, retrieval, feature_id, request.state.locale)
            return await enqueue(
                TaskDescriptor("conflict_review", "features", feature_id, "conflict_review"),
                {"locale": request.state.locale},
                idempotency_key=f"conflict-review:{feature_id}:{source_hash}",
                source_hash=source_hash,
                model_task="conflict_review",
            )
        except (WorkflowError, ValueError, OSError) as error:
            raise HTTPException(502, f"Conflict review failed: {error}") from error

    @app.get("/api/conflict-reviews/{run_id}")
    async def conflict_review_status(run_id: str) -> dict[str, object]:
        stored = workflow.conflict_review(run_id)
        if stored:
            return stored
        job = await job_repository.get(run_id)
        if not job or job.descriptor.task_kind != "conflict_review":
            raise HTTPException(404, "Conflict review not found")
        if job.status.value in {"completed", "awaiting_review"}:
            return job.result
        return {
            "run_id": job.id,
            "status": job.status.value,
            "phase": job.status.value,
            "progress": (job.progress_completed / job.progress_total if job.progress_total else 0),
            "recommended_state": "reviewing",
            "findings": [],
            "candidates": [],
        }

    @app.put("/api/conflict-reviews/{run_id}/resolutions")
    def resolve_conflict_review(run_id: str, data: ConflictResolutionsIn, request: Request) -> dict[str, object]:
        stored = workflow.conflict_review(run_id)
        if not stored:
            raise HTTPException(404, "Conflict review not found")
        try:
            feature_id = str(stored.get("feature_id", ""))
            current_query = conflict_review_query(workflow, retrieval, feature_id, request.state.locale)
            return workflow.resolve_conflict_review(
                run_id,
                [item.model_dump() for item in data.resolutions],
                current_query,
            )
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.delete("/api/conflict-reviews/{run_id}")
    async def cancel_conflict_review(run_id: str) -> dict[str, object]:
        job = await job_repository.request_cancel(run_id)
        if not job:
            raise HTTPException(404, "Conflict review not found")
        return {"run_id": job.id, "status": job.status.value, "recommended_state": "cancelled"}

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
    async def add_solution_progress(feature_id: str, data: SolutionProgressIn, request: Request) -> dict[str, object]:
        try:
            entry = workflow.add_solution_progress(feature_id, data.body, data.image_data, data.image_media_type)
            await enqueue_derived(
                "solution_progress_entries", str(entry["id"]), "body", data.body, request.state.locale
            )
            return entry
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/progress/{entry_id}/summarize-image", status_code=202)
    async def summarize_solution_image(entry_id: str, request: Request) -> JSONResponse:
        """Queue an image summary; the handler attaches it to the exact Work entry."""
        row = workflow.db.execute(
            "SELECT image_data,image_media_type FROM solution_progress_entries WHERE id=?", (entry_id,)
        ).fetchone()
        if not row:
            raise HTTPException(404, "Progress record not found")
        if not row[0]:
            raise HTTPException(400, "This progress record has no image")
        source_hash = digest(str(row[0]))
        return await enqueue(
            TaskDescriptor("image_summary", "solution_progress_entries", entry_id, "solution_work_summary"),
            {"locale": request.state.locale},
            idempotency_key=f"image-summary:{entry_id}:{source_hash}",
            source_hash=source_hash,
            model_task="image_summary",
        )

    @app.post("/api/progress/{entry_id}/comments", status_code=201)
    async def add_solution_comment(entry_id: str, data: SolutionCommentIn, request: Request) -> dict[str, object]:
        try:
            comment = workflow.add_solution_comment(entry_id, data.body)
            await enqueue_derived(
                "solution_progress_comments", str(comment["id"]), "body", data.body, request.state.locale
            )
            return comment
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/features/{feature_id}/checklist", status_code=201)
    async def add_solution_checklist(feature_id: str, data: SolutionChecklistIn, request: Request) -> dict[str, object]:
        try:
            item = workflow.add_solution_checklist_item(feature_id, data.body)
            await enqueue_derived("solution_checklist_items", str(item["id"]), "body", data.body, request.state.locale)
            return item
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
            return workflow.assess_importance(
                problem_id, data.alignment, data.impact, data.urgency, data.leverage, data.evidence
            )
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

    @app.post("/api/features/{feature_id}/completion-review", status_code=202)
    async def completion_review(feature_id: str, request: Request) -> JSONResponse:
        """Queue an evidence review; only the user may apply the subsequent decision."""
        board = workflow.board(request.state.locale)
        feature = next((item for item in board["features"] if item["id"] == feature_id), None)
        if not feature:
            raise HTTPException(404, "Solution not found")
        progress = workflow.solution_progress(feature_id, request.state.locale)
        source_hash = digest(json.dumps({"feature": feature, "progress": progress}, sort_keys=True, ensure_ascii=False))
        return await enqueue(
            TaskDescriptor("completion_review", "features", feature_id, "completion_review", "review_ready"),
            {"locale": request.state.locale},
            idempotency_key=f"completion-review:{feature_id}:{source_hash}",
            source_hash=source_hash,
            model_task="completion_review",
        )

    @app.post("/api/problems/{problem_id}/complete")
    async def complete_problem(problem_id: str, data: ProblemCompletionIn | None = None) -> dict[str, object]:
        try:
            workflow.complete_problem(problem_id, data.reason if data else "", data.review_id if data else "")
            result = write_completion_playbook(problem_id, refresh_lineage=True)
            result["report_job_id"] = await enqueue_completion_report(problem_id, refresh_lineage=False)
            return result
        except WorkflowError as error:
            raise HTTPException(400, str(error)) from error
        except OSError as error:
            raise HTTPException(409, f"Could not write completed-work Playbook: {error}") from error

    def ensure_completion_document_unmodified(problem_id: str) -> sqlite3.Row | None:
        return completion_archive.ensure_unmodified(problem_id)

    def current_problem_lineages(problem_id: str, refresh: bool) -> list[dict[str, object]]:
        return completion_archive.lineages(problem_id, refresh=refresh)

    def write_completion_playbook(problem_id: str, refresh_lineage: bool = False) -> dict[str, object]:
        return completion_archive.publish(problem_id, lineages=current_problem_lineages(problem_id, refresh_lineage))

    @app.post("/api/problems/{problem_id}/completion-playbook/regenerate", status_code=202)
    async def regenerate_completion_playbook(problem_id: str) -> JSONResponse:
        problem = workflow.db.execute("SELECT state FROM problems WHERE id=?", (problem_id,)).fetchone()
        if not problem or problem["state"] != "completed":
            raise HTTPException(400, "Only a completed Problem can regenerate its completed-work document")
        try:
            ensure_completion_document_unmodified(problem_id)
            job_id = await enqueue_completion_report(problem_id, refresh_lineage=True)
            job = await job_repository.get(job_id)
            assert job is not None
            return JSONResponse(job_view(job), status_code=202)
        except WorkflowError as error:
            raise HTTPException(409, str(error)) from error

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

    @app.post("/api/features/{feature_id}/lineage/regenerate")
    async def regenerate_feature_lineage(feature_id: str, data: LineageRegenerateIn) -> Response:
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
            if data.include_inference:
                source_hash = lineage_source_hash(workflow, feature_id)
                return await enqueue(
                    TaskDescriptor("lineage_inference", "features", feature_id, "solution_lineage"),
                    {"force": True},
                    idempotency_key=f"lineage-inference:{feature_id}:{source_hash}",
                    source_hash=source_hash,
                    model_task="lineage_inference",
                )
            lineage = workflow.create_lineage_snapshot(feature_id, force=True)
            projection = write_completion_playbook(str(feature["problem_id"]))
            return JSONResponse({**lineage, "document_sync": projection}, status_code=201)
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
        row = workflow.db.execute(
            "SELECT path,source_hash FROM completion_playbooks WHERE problem_id=?", (problem_id,)
        ).fetchone()
        if not row:
            raise HTTPException(404, "Completed-work document is not tracked for this Problem")
        path = str(row["path"])
        externally_modified = False
        try:
            externally_modified = digest(vault.read_text(path)) != str(row["source_hash"])
        except FileNotFoundError:
            pass
        if externally_modified and not force:
            raise HTTPException(
                409,
                "This completed-work document was modified outside LLM Wiki. Delete anyway to remove the document, Raw Data, and generated captures.",
            )
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
    async def patch_proposal(feature_id: str, data: PatchIn, request: Request) -> dict[str, str]:
        try:
            before = vault.read_text(data.path)
            content = data.content
            managed = "llm_wiki_managed: true" in before and "canonical_locale: en" in before
            if managed and request.state.locale == "ko":
                base_url, api_key, model = provider_settings.credentials("knowledge_translation")
                normalized = await fast_queue.complete_json(
                    base_url=base_url,
                    api_key=api_key,
                    model=model,
                    messages=[
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
                    schema_name="knowledge English normalization",
                )
                content = str(normalized.get("content", "")).strip()
                if not content:
                    raise ValueError("Knowledge English normalization was empty")
            patch = propose_section_patch(before, data.operation, data.heading, content)
            return workflow.save_patch_proposal(
                feature_id,
                data.path,
                data.operation,
                data.heading,
                content,
                patch.base_hash,
                patch.before,
                patch.proposed,
            )
        except (WorkflowError, ValueError, OSError) as error:
            raise HTTPException(400, str(error)) from error

    @app.post("/api/patches/{patch_id}/apply", status_code=204)
    def apply_patch(patch_id: str) -> None:
        try:
            stored = workflow.patch(patch_id)
            patch = SectionPatch(
                stored["operation"],
                stored["heading"],
                stored["content"],
                stored["base_hash"],
                stored["before_text"],
                stored["proposed_text"],
            )
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
            provider_settings.save(
                data.base_url,
                data.model,
                data.api_key,
                data.advanced_model,
                data.advanced_tasks,
                data.report_language,
                data.async_worker_count,
            )
            return provider_settings.public()
        except Exception as error:
            raise HTTPException(400, f"Could not save provider configuration: {error}") from error

    @app.post("/api/provider/test")
    async def provider_health() -> dict[str, object]:
        try:
            base_url, api_key, model = provider_settings.credentials("problem_enrichment")
            models = await fast_queue.models(base_url=base_url, api_key=api_key, model=model)
            return {"models": models, "configured_model": model}
        except Exception as error:  # provider failures must not affect local operation
            raise HTTPException(502, f"Provider health check failed: {error}") from error

    @app.post("/api/ai/enrich-problem")
    async def enrich_problem(data: EnrichIn, request: Request) -> dict[str, object]:
        try:
            base_url, api_key, model = provider_settings.credentials()
            statement = data.statement + "\n\n" + response_language_instruction(request.state.locale)
            result = await fast_queue.complete_json(
                base_url=base_url,
                api_key=api_key,
                model=model,
                messages=[
                    {
                        "role": "user",
                        "content": "Return JSON only with normalized_problem, pain, non_goals, categories, and importance_rationale. Use only the cited context. Never change workflow state or provide implementation steps.\nProblem: "
                        + statement
                        + "\nCitations: "
                        + ", ".join(data.citations[:8]),
                    }
                ],
                schema_name="problem enrichment",
            )
            required = {"normalized_problem", "pain", "non_goals", "categories", "importance_rationale"}
            if not required <= result.keys():
                raise ValueError("Problem enrichment response missed required fields")
            return {key: result[key] for key in required}
        except (ValueError, OSError) as error:
            raise HTTPException(502, str(error)) from error

    @app.get("/api/knowledge")
    async def read_knowledge(
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
            return {
                **base,
                "markdown": cached["translated_markdown"],
                "served_locale": "ko",
                "translated": True,
                "cache_status": "hit",
            }
        if not translate:
            return {**base, "markdown": canonical, "cache_status": "pending"}
        job = await job_repository.create(
            TaskDescriptor("knowledge_translation", "knowledge", path, "knowledge_document"),
            {"path": path, "locale": "ko", "entity_type": "knowledge", "entity_id": path},
            idempotency_key=f"knowledge-translation:{path}:{source_hash}:ko",
            source_hash=source_hash,
            model=provider_settings.model_for("knowledge_translation"),
        )
        return {**base, "markdown": canonical, "cache_status": "pending", "job_id": job.id}

    @app.get("/api/knowledge/translate", status_code=202)
    async def translate_knowledge(path: str = Query(min_length=1)) -> JSONResponse:
        try:
            canonical = vault.read_text(path)
        except (FileNotFoundError, ValueError) as error:
            raise HTTPException(404, "Knowledge document not found") from error
        if "llm_wiki_managed: true" not in canonical or "canonical_locale: en" not in canonical:
            raise HTTPException(409, "Knowledge document is not managed English canonical content")
        source_hash = digest(canonical)
        return await enqueue(
            TaskDescriptor("knowledge_translation", "knowledge", path, "knowledge_document"),
            {"path": path, "locale": "ko"},
            idempotency_key=f"knowledge-translation:{path}:{source_hash}:ko",
            source_hash=source_hash,
        )

    @app.post("/api/{entity_type}/{entity_id}/project", status_code=201)
    def project(entity_type: str, entity_id: str) -> dict[str, str]:
        try:
            path, content = workflow.projection(entity_type, entity_id)
            try:
                existing = vault.read_text(path)
                previous = retrieval.db.execute(
                    "SELECT source_hash FROM mirror_files WHERE entity_type=? AND entity_id=?", (entity_type, entity_id)
                ).fetchone()
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
        return FileResponse(
            Path(__file__).parent.parent / "static" / "index.html", headers={"Cache-Control": "no-store"}
        )

    return app
