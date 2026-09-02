from llm_wiki.services.markdown import parse_markdown


def test_obsidian_markdown_signals_are_extracted() -> None:
    doc = parse_markdown(
        "projects/roadmap.md",
        """---
aliases: [Plan 2026, roadmap]
tags:
  - work/strategy
---
# Direction
Use [[prior/decision#Keep it local|the decision]], ![[diagram.png]], and [[note#^evidence]].
- [ ] Review #urgent/today
```md
# not-a-heading
```
""",
    )
    assert doc.aliases == ("Plan 2026", "roadmap")
    assert doc.headings == ("Direction",)
    assert set(doc.tags) == {"work/strategy", "urgent/today"}
    assert doc.links[0].target == "prior/decision"
    assert doc.links[0].anchor == "Keep it local"
    assert doc.links[0].label == "the decision"
    assert doc.links[1].embed is True
    assert doc.links[2].block_id == "evidence"


def test_korean_unicode_and_canonical_metadata_round_trip() -> None:
    doc = parse_markdown(
        "Knowledge/결과.md",
        "---\nllm_wiki_managed: true\ncanonical_locale: en\naliases: [한국어 결과]\n---\n# English canonical\n한글 열람 메모와 [[결정/공유]]",
    )

    assert doc.path == "Knowledge/결과.md"
    assert doc.aliases == ("한국어 결과",)
    assert doc.headings == ("English canonical",)
    assert doc.links[0].target == "결정/공유"
