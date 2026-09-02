from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Link:
    target: str
    label: str | None = None
    anchor: str | None = None
    block_id: str | None = None
    embed: bool = False


@dataclass(frozen=True)
class ParsedDocument:
    path: str
    title: str
    content: str
    body: str
    frontmatter: dict[str, object]
    headings: tuple[str, ...]
    tags: tuple[str, ...]
    links: tuple[Link, ...]
    aliases: tuple[str, ...] = ()


@dataclass(frozen=True)
class SearchResult:
    path: str
    title: str
    snippet: str
    headings: tuple[str, ...]
    tags: tuple[str, ...]
    score: float
    source_hash: str
    matched_by: tuple[str, ...] = field(default_factory=tuple)
