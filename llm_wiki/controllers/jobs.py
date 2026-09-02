from __future__ import annotations

import asyncio
import json
from datetime import datetime
from typing import Any

from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import StreamingResponse

from llm_wiki.services.jobs import Job, JobNotification


router = APIRouter(prefix="/api")


def _time(value: datetime | None) -> str | None:
    return value.isoformat() if value else None


def job_view(job: Job) -> dict[str, Any]:
    return {
        "id": job.id,
        "task_kind": job.descriptor.task_kind,
        "entity_type": job.descriptor.entity_type,
        "entity_id": job.descriptor.entity_id,
        "status": job.status.value,
        "progress": {"completed": job.progress_completed, "total": job.progress_total},
        "result_interface": job.descriptor.result_interface,
        "error": {"code": job.error_code, "message": job.error_message} if job.error_code else None,
        "created_at": _time(job.created_at),
        "started_at": _time(job.started_at),
        "finished_at": _time(job.finished_at),
    }


def notification_view(item: JobNotification) -> dict[str, Any]:
    return {
        "id": item.id,
        "job_id": item.job_id,
        "kind": item.kind,
        "title": item.title,
        "target": item.target,
        "read_at": _time(item.read_at),
        "dismissed_at": _time(item.dismissed_at),
    }


@router.get("/jobs")
async def list_jobs(request: Request) -> dict[str, Any]:
    jobs = await request.app.state.job_repository.list()
    return {"jobs": [job_view(job) for job in jobs]}


@router.get("/jobs/events")
async def job_events(request: Request) -> StreamingResponse:
    async def events():
        previous = ""
        sequence = 0
        while not await request.is_disconnected():
            jobs = await request.app.state.job_repository.list()
            fingerprint = "|".join(f"{job.id}:{job.status.value}:{job.progress_completed}:{job.progress_total}" for job in jobs)
            if fingerprint != previous:
                sequence += 1
                yield f"id: {sequence}\nevent: jobs\ndata: {json.dumps({'sequence': sequence})}\n\n"
                previous = fingerprint
            await asyncio.sleep(0.5)

    return StreamingResponse(events(), media_type="text/event-stream", headers={"Cache-Control": "no-cache"})


@router.get("/jobs/{job_id}")
async def get_job(job_id: str, request: Request) -> dict[str, Any]:
    job = await request.app.state.job_repository.get(job_id)
    if not job:
        raise HTTPException(404, "AI job not found")
    return job_view(job)


@router.get("/jobs/{job_id}/result")
async def get_job_result(job_id: str, request: Request) -> dict[str, Any]:
    job = await request.app.state.job_repository.get(job_id)
    if not job:
        raise HTTPException(404, "AI job not found")
    return {"job_id": job.id, "status": job.status.value, "result_interface": job.descriptor.result_interface, "result": job.result}


@router.post("/jobs/{job_id}/cancel")
async def cancel_job(job_id: str, request: Request) -> dict[str, Any]:
    job = await request.app.state.job_repository.request_cancel(job_id)
    if not job:
        raise HTTPException(409, "AI job cannot be cancelled")
    return job_view(job)


@router.post("/jobs/{job_id}/retry")
async def retry_job(job_id: str, request: Request) -> dict[str, Any]:
    job = await request.app.state.job_repository.retry(job_id)
    if not job:
        raise HTTPException(409, "AI job cannot be retried")
    return job_view(job)


@router.get("/notifications")
async def list_notifications(request: Request, unread_only: bool = False) -> dict[str, Any]:
    items = await request.app.state.job_repository.notifications(unread_only=unread_only)
    return {"notifications": [notification_view(item) for item in items], "unread_count": sum(item.read_at is None and item.dismissed_at is None for item in items)}


@router.post("/notifications/{notification_id}/read")
async def read_notification(notification_id: str, request: Request) -> dict[str, Any]:
    item = await request.app.state.job_repository.update_notification(notification_id)
    if not item:
        raise HTTPException(404, "Notification not found")
    return notification_view(item)


@router.post("/notifications/{notification_id}/dismiss")
async def dismiss_notification(notification_id: str, request: Request) -> dict[str, Any]:
    item = await request.app.state.job_repository.update_notification(notification_id, dismiss=True)
    if not item:
        raise HTTPException(404, "Notification not found")
    return notification_view(item)
