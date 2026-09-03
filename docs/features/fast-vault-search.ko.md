# 빠른 Vault 검색

[English](fast-vault-search.md) | **한국어**

> **Resume where you left off.** 검색은 이전 Knowledge를 현재 결정으로 다시 가져옵니다.

![Vault 상대 경로와 일치한 노트 맥락을 보여주는 검색 결과](images/01-search-vault.png)

Obsidian 호환 Markdown의 경로, frontmatter, 별칭, 제목, 중첩 태그, wikilink, 참조, embed, 본문을
인덱싱합니다. 파일 변경은 로컬 SQLite FTS 인덱스에 반영되고 결과는 페이지 단위로 이어집니다.

Semantic 재정렬은 명시적으로 선택할 때 사용합니다. 네이티브 데스크톱 앱은 고정된 다국어
MiniLM ONNX 모델을 포함하고 시작 인덱싱 중 Rust에서 임베딩을 만듭니다. 모델 다운로드나 Python
Runtime은 필요하지 않습니다. 구조 검색은 모델·임베딩 장애 중에도 기존 Knowledge를 찾는 빠른
로컬 fallback으로 남습니다.

관련 Spec Kit: [001 — Fast Vault Search](../../specs/001-fast-vault-search/spec.md)
