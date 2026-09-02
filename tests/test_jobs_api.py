from __future__ import annotations

import asyncio
from pathlib import Path

from fastapi.testclient import TestClient

from llm_wiki.web.app import create_app
from llm_wiki.services.jobs import TaskDescriptor


def test_jobs_api_lists_safe_state_and_supports_cancel_retry(tmp_path: Path) -> None:
    app = create_app(tmp_path, tmp_path / "jobs-api.sqlite")
    with TestClient(app) as client:
        job = asyncio.run(app.state.job_repository.create(TaskDescriptor("embedding_refresh", result_interface="none"), {"private": "not returned"}))
        listed = client.get("/api/jobs").json()["jobs"]
        assert listed[0]["id"] == job.id
        assert "input" not in listed[0]
        assert client.post(f"/api/jobs/{job.id}/cancel").json()["status"] == "cancelled"
        assert client.get(f"/api/jobs/{job.id}/result").json()["result"] == {}


def test_notifications_api_has_stable_unread_count(tmp_path: Path) -> None:
    app = create_app(tmp_path, tmp_path / "noti-api.sqlite")
    with TestClient(app) as client:
        job = asyncio.run(app.state.job_repository.create(TaskDescriptor("completion_review", "features", "f1"), {}))
        notification = asyncio.run(app.state.job_repository.publish_notification(job.id, "review_ready", "Review ready", {"feature_id": "f1"}))
        unread = client.get("/api/notifications", params={"unread_only": "true"}).json()
        assert unread["unread_count"] == 1
        client.post(f"/api/notifications/{notification.id}/read")
        assert client.get("/api/notifications", params={"unread_only": "true"}).json()["unread_count"] == 0


def test_notification_dismissal_is_persisted_and_missing_ids_are_safe(tmp_path: Path) -> None:
    app = create_app(tmp_path, tmp_path / "noti-dismiss.sqlite")
    with TestClient(app) as client:
        job = asyncio.run(app.state.job_repository.create(TaskDescriptor("completion_review", "features", "f1"), {}))
        notification = asyncio.run(app.state.job_repository.publish_notification(job.id, "review_ready", "Review ready", {"feature_id": "f1"}))
        dismissed = client.post(f"/api/notifications/{notification.id}/dismiss")
        assert dismissed.status_code == 200
        assert dismissed.json()["dismissed_at"]
        assert client.get("/api/notifications").json()["unread_count"] == 0
        assert client.post("/api/notifications/missing/read").status_code == 404
        assert client.post("/api/notifications/missing/dismiss").status_code == 404
