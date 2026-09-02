from __future__ import annotations

from typing import Any

from llm_wiki.core.jobs import StaleJobError, TaskDescriptor
from llm_wiki.services.handlers.provider import ProviderBackedHandler, ProviderFactory
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.handlers.targets import job_target, require_source_hash
from llm_wiki.services.handlers.validation import validate_bilingual_image_summary
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


class ImageSummaryHandler(ProviderBackedHandler):
    descriptor = TaskDescriptor("image_summary", result_interface="solution_work_summary")

    def __init__(
        self,
        workflow: WorkflowEngine,
        settings: ProviderSettings,
        provider_factory: ProviderFactory | None = None,
    ):
        super().__init__(settings, provider_factory)
        self.workflow = workflow

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(self.descriptor, self)

    async def __call__(self, context: HandlerContext) -> dict[str, Any]:
        _entity_type, entry_id = job_target(context)
        locale = str(context.payload.get("locale", "en"))
        row = self.workflow.db.execute(
            "SELECT image_data,image_media_type,feature_id FROM solution_progress_entries WHERE id=?",
            (entry_id,),
        ).fetchone()
        if not row or not row[0]:
            raise WorkflowError("Progress image is no longer available")
        require_source_hash(context, digest(str(row[0])))
        provider = self.provider_factory("image_summary")
        try:
            result = await provider.complete_json(
                [{"role": "user", "content": self._content(str(row[0]), str(row[1] or "image/png"))}],
                "progress image summary",
            )
        finally:
            await provider.aclose()
        current = self.workflow.db.execute(
            "SELECT image_data FROM solution_progress_entries WHERE id=?",
            (entry_id,),
        ).fetchone()
        if not current:
            raise StaleJobError("Progress entry was removed while its summary was running")
        require_source_hash(context, digest(str(current[0])))
        versions = validate_bilingual_image_summary(result)
        self.workflow.set_solution_progress_summaries(entry_id, versions, locale)
        return {
            "summary": versions[locale]["image_summary"],
            "localized_versions": versions,
            "missing_locales": [],
            "entry_id": entry_id,
            "feature_id": str(row[2]),
        }

    @staticmethod
    def _content(image_data: str, media_type: str) -> list[object]:
        return [
            {
                "type": "text",
                "text": (
                    'Return JSON only with exactly this shape: {"ko":{"summary":string},'
                    '"en":{"summary":string}}. Summarize this work-progress image accurately and concisely in '
                    "natural Korean and English in this one response. Both versions must describe the same visible "
                    "evidence. Do not infer invisible details."
                ),
            },
            {"type": "image_url", "image_url": {"url": f"data:{media_type};base64,{image_data}"}},
        ]
