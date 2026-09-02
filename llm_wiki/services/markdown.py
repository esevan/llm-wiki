from __future__ import annotations

import hashlib
import re
from pathlib import Path

from llm_wiki.core.models import Link, ParsedDocument

_FRONTMATTER = re.compile(r"\A---\s*\n(.*?)\n---\s*\n?", re.DOTALL)
_HEADING = re.compile(r"^#{1,6}\s+(.+?)\s*$", re.MULTILINE)
_TAG = re.compile(r"(?<![\w/])#([\w-]+(?:/[\w-]+)*)", re.UNICODE)
_LINK = re.compile(r"(!)?\[\[([^\]]+)\]\]")
_CODE_FENCE = re.compile(r"^```.*?^```[ \t]*$", re.MULTILINE | re.DOTALL)


def _scalar(value: str) -> object:
    value = value.strip().strip('"\'')
    if value.lower() in {"true", "false"}:
        return value.lower() == "true"
    if value.startswith("[") and value.endswith("]"):
        return [part.strip().strip('"\'') for part in value[1:-1].split(",") if part.strip()]
    return value


def parse_frontmatter(text: str) -> tuple[dict[str, object], str]:
    match = _FRONTMATTER.match(text)
    if not match:
        return {}, text
    data: dict[str, object] = {}
    active_list: str | None = None
    for raw in match.group(1).splitlines():
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        if raw.startswith("  - ") and active_list:
            data.setdefault(active_list, [])
            assert isinstance(data[active_list], list)
            data[active_list].append(_scalar(raw[4:]))
        elif ":" in raw:
            key, value = raw.split(":", 1)
            active_list = key.strip()
            data[active_list] = _scalar(value) if value.strip() else []
    return data, text[match.end():]


def parse_wikilink(raw: str, embed: bool = False) -> Link:
    target_label, *label = raw.split("|", 1)
    target, sep, anchor = target_label.strip().partition("#")
    block_id = anchor[1:] if anchor.startswith("^") else None
    return Link(target=target, label=label[0].strip() if label else None,
                anchor=anchor if sep and not block_id else None, block_id=block_id, embed=embed)


def parse_markdown(path: str, text: str) -> ParsedDocument:
    frontmatter, body = parse_frontmatter(text)
    searchable_body = _CODE_FENCE.sub("", body)
    headings = tuple(re.sub(r"\s+#*$", "", h).strip() for h in _HEADING.findall(searchable_body))
    links = tuple(parse_wikilink(m.group(2), bool(m.group(1))) for m in _LINK.finditer(searchable_body))
    tags = set(_TAG.findall(searchable_body))
    front_tags = frontmatter.get("tags", [])
    if isinstance(front_tags, str):
        front_tags = [front_tags]
    if isinstance(front_tags, list):
        tags.update(str(t).lstrip("#") for t in front_tags)
    aliases = frontmatter.get("aliases", [])
    if isinstance(aliases, str):
        aliases = [aliases]
    return ParsedDocument(path=path, title=Path(path).stem, content=text, body=body,
                          frontmatter=frontmatter, headings=headings, tags=tuple(sorted(tags)),
                          links=links, aliases=tuple(str(a) for a in aliases if a))


def content_hash(content: str) -> str:
    return hashlib.sha256(content.encode("utf-8")).hexdigest()
