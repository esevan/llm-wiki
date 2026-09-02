from __future__ import annotations

import sqlite3
import json


DEFAULT_BASE_URL = "http://127.0.0.1:8317/v1"
SERVICE_NAME = "LLM Wiki OpenAI Provider"
ACCOUNT_NAME = "default"


ADVANCED_TASK_DEFAULTS = {
    "capture_assistance": True,
    "problem_drafting": True,
    "problem_assistance": True,
    "workbench_organization": False,
    "solution_drafting": True,
    "solution_assistance": True,
    "completed_solution_chat": False,
    "conflict_review": True,
    "image_summary": True,
    "completion_review": True,
    "completion_report": True,
    "lineage_inference": True,
    "problem_enrichment": False,
    "knowledge_translation": False,
}


class ProviderSettings:
    """Endpoint/model metadata in SQLite; secret exclusively in OS keyring."""

    def __init__(self, db: sqlite3.Connection):
        self.db = db
        self.db.execute("CREATE TABLE IF NOT EXISTS provider_settings (id INTEGER PRIMARY KEY CHECK(id=1), base_url TEXT NOT NULL, model TEXT NOT NULL DEFAULT '', advanced_model TEXT NOT NULL DEFAULT '', advanced_tasks TEXT NOT NULL DEFAULT '{}', report_language TEXT NOT NULL DEFAULT 'ko', async_worker_count INTEGER NOT NULL DEFAULT 2)")
        columns = {row[1] for row in self.db.execute("PRAGMA table_info(provider_settings)")}
        if "advanced_model" not in columns:
            self.db.execute("ALTER TABLE provider_settings ADD COLUMN advanced_model TEXT NOT NULL DEFAULT ''")
        if "advanced_tasks" not in columns:
            self.db.execute("ALTER TABLE provider_settings ADD COLUMN advanced_tasks TEXT NOT NULL DEFAULT '{}'")
        if "report_language" not in columns:
            self.db.execute("ALTER TABLE provider_settings ADD COLUMN report_language TEXT NOT NULL DEFAULT 'ko'")
        if "async_worker_count" not in columns:
            self.db.execute("ALTER TABLE provider_settings ADD COLUMN async_worker_count INTEGER NOT NULL DEFAULT 2")
        self.db.execute("INSERT OR IGNORE INTO provider_settings(id,base_url,model,advanced_model,advanced_tasks,report_language,async_worker_count) VALUES (1,?, '', '', '{}', 'ko', 2)", (DEFAULT_BASE_URL,))
        self.db.commit()

    def public(self) -> dict[str, object]:
        row = self.db.execute("SELECT base_url,model,advanced_model,advanced_tasks,report_language,async_worker_count FROM provider_settings WHERE id=1").fetchone()
        try:
            saved_tasks = json.loads(row[3])
        except (TypeError, json.JSONDecodeError):
            saved_tasks = {}
        advanced_tasks = {task: bool(saved_tasks.get(task, default)) for task, default in ADVANCED_TASK_DEFAULTS.items()}
        return {"base_url": row[0], "model": row[1], "advanced_model": row[2], "advanced_tasks": advanced_tasks, "report_language": row[4] or "ko", "async_worker_count": int(row[5]), "api_key_configured": bool(self._secret())}

    def save(self, base_url: str, model: str, api_key: str | None, advanced_model: str = "", advanced_tasks: dict[str, bool] | None = None, report_language: str | None = None, async_worker_count: int | None = None) -> None:
        normalized = {task: bool(selected) for task, selected in (advanced_tasks or {}).items() if task in ADVANCED_TASK_DEFAULTS}
        language = (report_language or str(self.public().get("report_language", "ko"))).strip().lower()
        if language not in {"ko", "en"}:
            raise ValueError("Report language must be 'ko' or 'en'")
        count = int(async_worker_count if async_worker_count is not None else self.public().get("async_worker_count", 2))
        if not 1 <= count <= 32:
            raise ValueError("Async worker count must be between 1 and 32")
        self.db.execute("UPDATE provider_settings SET base_url=?,model=?,advanced_model=?,advanced_tasks=?,report_language=?,async_worker_count=? WHERE id=1", (base_url.rstrip("/"), model.strip(), advanced_model.strip(), json.dumps(normalized), language, count))
        self.db.commit()
        if api_key:
            self._keyring().set_password(SERVICE_NAME, ACCOUNT_NAME, api_key)

    def credentials(self, task: str | None = None) -> tuple[str, str, str]:
        setting = self.public()
        secret = self._secret()
        if not secret:
            raise ValueError("Configure an API key in AI setup before using AI")
        use_advanced = bool(setting["advanced_tasks"].get(task, False))
        model = str((setting["advanced_model"] if use_advanced else "") or setting["model"])
        if not model:
            raise ValueError("Select a model in AI setup before using AI")
        return str(setting["base_url"]), secret, model

    def model_for(self, task: str | None = None) -> str:
        setting = self.public()
        use_advanced = bool(setting["advanced_tasks"].get(task, False))
        return str((setting["advanced_model"] if use_advanced else "") or setting["model"])

    def report_language(self) -> str:
        """Return the legacy preference; canonical Knowledge generation ignores it."""
        return str(self.public().get("report_language", "ko"))

    @staticmethod
    def _keyring():
        import keyring
        return keyring

    def _secret(self) -> str | None:
        try:
            return self._keyring().get_password(SERVICE_NAME, ACCOUNT_NAME)
        except Exception:
            return None
