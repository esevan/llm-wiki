from __future__ import annotations

from collections.abc import Callable

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider
from llm_wiki.services.settings import ProviderSettings

ProviderFactory = Callable[[str], AsyncOpenAICompatibleProvider]


class ProviderBackedHandler:
    """Shared provider construction for independently registered AI handlers."""

    def __init__(self, settings: ProviderSettings, provider_factory: ProviderFactory | None = None):
        self.settings = settings
        self.provider_factory = provider_factory or self._provider

    def _provider(self, task: str) -> AsyncOpenAICompatibleProvider:
        base_url, api_key, model = self.settings.credentials(task)
        return AsyncOpenAICompatibleProvider.with_client(base_url, api_key, model)
