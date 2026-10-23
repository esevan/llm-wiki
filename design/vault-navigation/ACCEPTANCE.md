# Vault navigation and reader — design proposal, approval pending

Scope: isolated HTML/CSS mocks only. No production behavior, application code, dependencies, or tests changed. Compass and Sessions are outside scope. Open `folders.html` locally; all assets are bundled, with no CDN requirement. Links switch mock states; folder expansion, filtering, retry and source opening are visual controls only.

## Review entry points

- `folders.html`: 1440×1000, Folders and selected white note.
- `moc.html`: 1440×1000, MOC index and linked overview.
- `folders.html`: 390×844, narrow note reader; back link opens `narrow-folders.html` with full navigation.
- `search.html`: search results with current result and same reader.
- `empty.html`, `loading.html`, `error.html`, `media-error.html`: explicit design states.
- Scroll `folders.html` to see the real image, table, and code sample. Breadcrumb remains visible.

## Exact implementation acceptance checklist (after mock approval)

### Layout and typography

- [ ] At 1440px: global rail 176px; browser 318px; reader fills remaining 946px. No window horizontal overflow.
- [ ] At 651–900px: rail 76px, browser 260px, remaining reader; rail labels hidden without losing accessible names.
- [ ] At ≤650px: reader occupies full viewport; navigation becomes separate full-width list. Visible back link restores selected tab, folder, filter and list scroll. Selecting a note restores reader and note scroll independently.
- [ ] Shell height 100vh. Browser tree and reader scroll independently; breadcrumb stays sticky at reader top. No body scroll trap. Long file names wrap.
- [ ] Reader surface #fff. Article outer max-width 776px with 44px horizontal padding, yielding 688px content. At ≤900px padding 24px; at ≤650px padding 22px. Paragraph line-height 1.8; body 15px; lead 17px; desktop title 34px/1.4; mobile title 28px.
- [ ] Use existing Nunito/Noto Sans KR body and DM Mono metadata tokens. Bundled mock Nunito 500 is assigned regular weight; Korean uses system fallback in the mock. Production must use the existing complete bundled font setup.
- [ ] Cream #fffaf2 shell, ink #27242a, muted #7d7780, border #eee5e8, accent #ff2b85; selected text #b91860 with #fff0f6 surface. Shared token changes require separate review.
- [ ] Tabs: 4px inset/gap, 12px container radius, 9px tab radius; active white surface. Row 11px/10px padding, 9px radius. Sticky breadcrumb ≥62px high with 19px/30px padding.

### Navigation and reader

- [ ] Search / MOC / Folders are equally discoverable persistent tabs. Search retains current keyword and semantic-search control from the existing experience (semantic control is not drawn in this proposal; retain it in the final search form).
- [ ] Folders show actual Vault hierarchy with disclosure buttons and note counts. Selected note has tinted surface, strong weight and aria-current. Folder disclosure selection and note selection are distinct. No automatic publication or file move.
- [ ] MOC shows actual authored index notes and counts of resolved outgoing note links; do not fabricate an AI-generated map or rank users. MOC contents open in the same reader; linked notes preserve return context.
- [ ] Breadcrumb shows actual relative folder/file path; title and last-updated metadata reflect selected source. Source button opens original Markdown without modifying it. Do not display synthetic read time unless computed.
- [ ] Support headings, paragraphs, lists, emphasis, blockquotes, links, GFM tables, fenced code and local images. Preserve canonical Markdown and existing translation boundary. The sample Korean note is fixture content, not an automatic canonical translation.
- [ ] Mermaid fenced blocks render diagrams in a bounded white/cream frame. Diagram must remain readable via horizontal scroll or accessible zoom when needed, rather than shrinking labels indefinitely on narrow screens. Narrow mock demonstrates the frame; final fit/zoom requires implementation validation.
- [ ] The mock diagram is a hand-authored SVG visual reference corresponding to the included Mermaid source, not a functioning Mermaid engine. The image is an actual repository documentation JPEG, loaded via an image element. Production Markdown/Mermaid/image rendering and resource safety remain unimplemented and unverified.
- [ ] Clicking an image provides a full-size view with a close/back control; preserve article scroll and provide descriptive alt text. Images never stretch beyond article width. Tables and code scroll within their own containers.

### States and accessibility

- [ ] Loading keeps navigation, selected file and breadcrumb stable; announce loading once, allow cancellation, and honor reduced motion. No fake progress percentage.
- [ ] Empty folder shows empty-folder wording and another-folder action; no-results search separately offers clear filters. No selected note shows an invitation to choose a note.
- [ ] File access error keeps selected context and offers retry; moved/missing path must not silently open another note. Diagram and image failure stay inline and leave the rest of the note readable. Mermaid failure exposes escaped source; broken images retain alt text/path.
- [ ] Success is shown through note content and published metadata; no distracting toast on every note selection.
- [ ] Controls include hover tint, pressed feedback, 3px #d91969 focus ring with 2px offset, disabled opacity .55. Avoid color-only selection and status. Tab/arrow key behavior follows tablist pattern; tree disclosure buttons expose aria-expanded. All controls have descriptive names.
- [ ] Verify Korean and English long strings, keyboard traversal, 200% zoom, reduced motion, and selection/scroll restoration in implementation. Mock HTML tab links are navigation links, not a complete ARIA tab widget; fix semantics when implementing.

## Approval gate and verification

User must approve these mocks before production implementation. Approval must identify the accepted screenshots/revision; implementation should compare the same viewport sizes against those accepted images. Differences to widths, typography, colors, hierarchy, narrow navigation, Markdown/media treatment or state wording require review before claiming parity.

Documentation guide reviewed. Current feature guides are intentionally unchanged because this is a proposal, not shipped behavior. This artifact records the unfinished approval and implementation work without rewriting another worktree's continuation context.

E2E skipped: isolated static design files do not change runtime, persistence, native boundaries or packaging. No release build, npm test, or Cargo test is justified for this proposal. Visual rendering, state inspection and whitespace validation are the relevant checks. Native Markdown resource handling, Mermaid parsing, navigation state, and accessibility behavior need focused implementation checks after approval.

## Render review record — 2026-10-23

Rendered with a nonpersistent local WKWebView, not the installed production application. Manually inspected actual PNG pixels for wide Folders, MOC, narrow note, narrow folder list, scrolled image/table/source, empty, file-error and inline media-error states. Initial narrow Korean title split mid-word and diagram labels became too small; corrected keep-all wrapping and 560px minimum diagram canvas with local horizontal scroll. Made sticky breadcrumb opaque so scrolled text does not bleed behind it, and reserved pink text for selected rows.

Captured dimensions: wide 1440×1000; narrow 390×844. Final screenshots are under `screenshots/`. These establish proposal appearance, not functional runtime verification. English expansion, real keyboard tab/arrow handling, 200% zoom and screen-reader behavior were not visually/behaviorally verified. No production implementation is authorized yet.

Library blocker: used the current Library skill's three complete helper files and its ordered prepared-upload batch for folders-wide.png, moc-wide.png and note-narrow.png. System Python 3.9 could not run the helper; Homebrew Python 3.14 resolved that. Sandboxed helper failed DNS before upload. Supported escalated retry reached the service and failed `Library prepare_uploads is not available`. No successful preparation/finalization or Library file IDs were returned. No direct-upload fallback attempted after the prepared flow started. Local screenshots remain the authoritative deliverables; Mac paths must not be assumed accessible from cloud tools.

### Library delivery resolved

The parent explicitly authorized the currently available direct local-file create route after the prepared helper availability failure. The direct tool successfully uploaded all three Mac PNGs; each per-item status was `succeeded`, version 0. Local Library identity metadata was persisted using the current helper.

| Screenshot | Verified library_file_id | file_id |
| --- | --- | --- |
| folders-wide.png | libfile_fd9c94ce3404819189f3ffa03a7e81f7 | file_000000005bd081f592a7d7f20f4befe8 |
| moc-wide.png | libfile_c73824b0b1b88191bf345885ae0ab797 | file_00000000315481f5af0d1859449ffc42 |
| note-narrow.png | libfile_ccb40bab39cc819192ea0acf92796ee2 | file_00000000f95881f58ad154bc67c6a133 |

## Implementation revision

The user subsequently approved these mockups and authorized implementation in this same worktree. The historical design proposal above remains the approval reference. See [implementation review and remaining checks](IMPLEMENTATION.md) and [verified implementation screenshot identities](library-delivery.json).
