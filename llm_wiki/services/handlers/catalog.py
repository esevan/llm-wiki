from __future__ import annotations

from llm_wiki.services.handlers.completion_review import CompletionReviewHandler
from llm_wiki.services.handlers.derived_translation import DerivedTranslationHandler
from llm_wiki.services.handlers.drafting import WorkflowDraftHandler
from llm_wiki.services.handlers.image_summary import ImageSummaryHandler
from llm_wiki.services.handlers.knowledge_translation import KnowledgeTranslationHandler
from llm_wiki.services.handlers.provider import ProviderFactory
from llm_wiki.services.handlers.refinement import WorkflowRefinementHandler
from llm_wiki.services.handlers.registry import HandlerRegistry
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.vault import MarkdownVaultAdapter
from llm_wiki.services.workflow import WorkflowEngine


def register_workflow_handlers(
    registry: HandlerRegistry,
    workflow: WorkflowEngine,
    settings: ProviderSettings,
    provider_factory: ProviderFactory | None = None,
) -> None:
    for handler in (
        WorkflowDraftHandler(workflow, settings, provider_factory),
        WorkflowRefinementHandler(workflow, settings, provider_factory),
        ImageSummaryHandler(workflow, settings, provider_factory),
        CompletionReviewHandler(workflow, settings, provider_factory),
    ):
        handler.register(registry)


def register_translation_handlers(
    registry: HandlerRegistry,
    workflow: WorkflowEngine,
    settings: ProviderSettings,
    vault: MarkdownVaultAdapter,
    provider_factory: ProviderFactory | None = None,
) -> None:
    KnowledgeTranslationHandler(workflow, settings, vault, provider_factory).register(registry)
    DerivedTranslationHandler(workflow, settings, provider_factory).register(registry)
