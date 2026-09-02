from __future__ import annotations

from datetime import date
from typing import Any

from llm_wiki.services.lineage import readable_report_context
from llm_wiki.services.localization import VaultKnowledgeTranslationCache
from llm_wiki.services.patches import digest
from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.vault import MarkdownVaultAdapter
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


class CompletionArchivePublisher:
    """Synchronous deterministic publication; AI content is supplied by a Handler."""

    def __init__(
        self,
        workflow: WorkflowEngine,
        retrieval: RetrievalEngine,
        vault: MarkdownVaultAdapter,
        translations: VaultKnowledgeTranslationCache,
    ):
        self.workflow = workflow
        self.retrieval = retrieval
        self.vault = vault
        self.translations = translations

    def lineages(self, problem_id: str, *, refresh: bool = False) -> list[dict[str, Any]]:
        feature_ids = [
            str(row[0])
            for row in self.workflow.db.execute(
                "SELECT id FROM features WHERE problem_id=? ORDER BY created_at,rowid", (problem_id,)
            )
        ]
        values: list[dict[str, Any]] = []
        for feature_id in feature_ids:
            if refresh:
                lineage = self.workflow.create_lineage_snapshot(feature_id, force=True)
                values.append(self.workflow.mark_lineage_inference_complete(feature_id, str(lineage["snapshot_id"])))
                continue
            try:
                values.append(self.workflow.lineage(feature_id))
            except WorkflowError:
                lineage = self.workflow.create_lineage_snapshot(feature_id, force=False)
                values.append(self.workflow.mark_lineage_inference_complete(feature_id, str(lineage["snapshot_id"])))
        return values

    def ensure_unmodified(self, problem_id: str):
        existing = self.retrieval.db.execute(
            "SELECT path,source_hash FROM completion_playbooks WHERE problem_id=?", (problem_id,)
        ).fetchone()
        if existing:
            try:
                if digest(self.vault.read_text(str(existing["path"]))) != existing["source_hash"]:
                    raise WorkflowError(
                        "Completed-work Playbook was modified externally; review it before regenerating"
                    )
            except FileNotFoundError:
                pass
        return existing

    def publish(
        self,
        problem_id: str,
        *,
        lineages: list[dict[str, Any]] | None = None,
        executive_summary: str = "",
        report_body: str = "",
        status: str = "deterministic_fallback",
    ) -> dict[str, Any]:
        existing = self.ensure_unmodified(problem_id)
        directory = (
            str(existing["path"]).rsplit("/", 1)[0] if existing else f"{date.today().year}/90. Archive/Completed Work"
        )
        lineages = lineages if lineages is not None else self.lineages(problem_id)
        report_input_hash = digest(readable_report_context(lineages))
        raw_path, raw_content = self.workflow.completion_playbook(problem_id, directory, raw=True, lineages=lineages)
        path, content = self.workflow.completion_playbook(
            problem_id, directory, executive_summary=executive_summary, report_body=report_body, lineages=lineages
        )
        if existing:
            old_path = str(existing["path"])
            if old_path != path:
                if (self.vault.root / path).exists():
                    raise WorkflowError("A completed-work document already uses this human-readable name")
                self.vault.move(old_path, path)
        self.vault.atomic_write(path, content)
        self.translations.invalidate(path)
        self.vault.atomic_write(raw_path, raw_content)
        for asset_path, asset_content in self.workflow.completion_assets(problem_id, directory):
            self.vault.atomic_write_bytes(asset_path, asset_content)
        selected = lineages[-1] if lineages else None
        self.workflow.remember_completion_playbook(
            problem_id,
            path,
            digest(content),
            str(selected["snapshot_id"]) if selected else "",
            int(selected["version"]) if selected else 0,
            report_input_hash,
            status,
        )
        self.retrieval.index_changed()
        return {
            "path": path,
            "raw_path": raw_path,
            "lineage": {
                "snapshot_id": selected["snapshot_id"],
                "status": selected["status"],
                "version": selected["version"],
                "retryable": bool(selected.get("generation", {}).get("inference_error")),
            }
            if selected
            else None,
            "report_generation": {
                "status": status,
                "lineage_snapshot_id": selected["snapshot_id"] if selected else "",
                "lineage_version": selected["version"] if selected else 0,
            },
        }
