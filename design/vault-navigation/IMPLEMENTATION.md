# Vault navigation implementation handoff

Approved design: [acceptance contract](ACCEPTANCE.md), original [Folders](screenshots/folders-wide.png), [MOC](screenshots/moc-wide.png), and [narrow note](screenshots/note-narrow.png). Implementation stays on the separate `design/vault-navigation-mockups` branch; no merge or push is authorized.

## Delivered behavior

Search retains semantic/lexical retrieval, stale-response guards and 20-result pagination. Search, authored MOC indexes and live folders share an inline white reader. Folder disclosure is independent from selection. Filtering, selected paths, open folders, note history and independent scroll positions stay in the mounted view. Empty folders, loading, cancellation, search/read retries and inline diagram/image errors have visible states. A read-only Markdown source view and a closable, focus-contained full-size image view are available.

Real Mermaid renders locally with strict locked configuration; note-controlled init directives, remote resource directives, oversized diagrams and active SVG content are rejected or sanitized. Markdown never executes embedded HTML. Note links and aliased wikilinks resolve only against known Vault notes; short ambiguous names remain inert. Local raster images are read through the native command boundary, canonicalized inside the configured Vault, bounded to 10 MiB, and checked for PNG/JPEG/GIF/WebP signatures. Standard Markdown image syntax is supported; Obsidian image-transclusion syntax is not interpreted. Canonical/original notes are bounded to 10 MiB. This reader never initiates translation, writes notes or publishes Knowledge.

## Visual verification

Actual shipping SearchView and Sidebar were rendered with shared CSS, all bundled font declarations and the production Tauri CSP in a disposable nonpersistent WKWebView. The API data adapter is synthetic, not a personal Vault and not the packaged application. The fixture uses the real Mermaid engine, the production Markdown component and a real project JPEG. A native application-command integration test separately verifies browse, canonical note and relative image access.

Pixel inspection covered wide Folders, MOC, 390px reader/list, scroll to the real image/table, file errors, malformed Mermaid, missing image, empty folder, loading, full-size image, exposed Mermaid source and wikilink navigation. Korean and English UI were rendered. Side-by-side comparisons at original dimensions are [wide](implementation/compare-wide.png), [MOC](implementation/compare-moc.png), and [narrow](implementation/compare-narrow.png); approved mock is left, implementation is right.

Computed DOM evidence in the adjacent JSON files confirms:

| Geometry / content | Wide 1440×1000 | Narrow 390×844 |
| --- | --- | --- |
| Body scroll width | 1440 | 390 |
| Global rail / browser / reader | 176 / 318 / 946 | 0 / 0 / 390 |
| Article outer width / padding | 776 / 44 each side | 390 / 22 each side |
| Actual Mermaid SVG count | 1 | 1 |
| Actual loaded image natural width | 1356 | 1356 |
| Embedded note scripts / iframes | 0 | 0 |
| Font loading | loaded | loaded |

The approved cream navigation, white note, pink selection, readable max width, sticky breadcrumb and responsive separation are retained. Source content supplies actual file names, counts, dates and diagram shapes. The static mock used system Korean fallback; implementation uses the required bundled Noto Sans KR, so glyph shapes/wrapping differ slightly. Mermaid's real SVG shapes replace the mock's hand-drawn SVG; diagram source disclosure and horizontal-scroll guidance are functional additions already required by the contract. The existing global language selector remains available in the rail; the isolated mock did not draw it. MOC list rows show actual paths rather than inventing descriptions/counts for unread notes. No read-time value is fabricated. These are recorded differences; pixel-identical text/content is not claimed.

## Verification

- `npm test`: 65 files, 496 tests passed; desktop helper, fake-provider, macOS signing/runtime, runtime parse, native-only and work-tracking boundary checks also passed.
- `npm run lint`, `npm run typecheck`, `npm run build`: passed. Production IIFE is approximately 6.95 MB uncompressed / 1.92 MB gzip with the local Mermaid engine. Bundled font verification passed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: full library run had 333 pass, 3 fail, 3 ignored. Sandbox-only loopback failures were removed by the supported approval flow. Two existing five-second Task execution burst tests passed on isolated rerun (4.19s and 4.50s). The unchanged structural-index timing test still failed alone at approximately 4.03s against a 3s limit; no unrelated performance threshold or execution code was changed.
- `cargo test --manifest-path src-tauri/Cargo.toml --test application_commands`: 14 passed, including the added real Vault command integration.
- `cargo test --manifest-path src-tauri/Cargo.toml vault_navigation`: 2 new native unit tests and 1 application-command integration passed. Cases cover live folders/MOCs, path traversal, parent paths inside the Vault, symlink escape, rejected SVG and size bounds.
- `git diff --check`: passed.

Packaged desktop E2E and release build were skipped. The remaining risk is bounded read-only navigation/rendering: native command integration exercises actual file boundaries, adapter/unit tests exercise request mapping and races, and the production-CSP WKWebView fixture exercises actual Mermaid/image rendering. These cheaper checks cover the concrete risks without an expensive packaged run; no persistence, lifecycle or packaging behavior was changed. Residual limits: no personal Vault inspection, no installed-app IPC capture, no 200% zoom or screen-reader audit, no cross-platform rendered review. The desktop currently enforces a 900px window minimum; 390px is fixture evidence. The native timing failure remains visible and unresolved.

Dependency audit reports six existing development-tool findings (`vite`, `vitest` and their tooling dependencies); the new Mermaid/DOMPurify dependency trees were not listed as vulnerable. No broad audit fix was applied.

## Reproduce the rendering fixture on macOS

Run from the dedicated worktree root. Create `.tmp/vault-render`, then bundle `frontend/src/test/vaultNavigation.fixture.tsx` with the installed esbuild:

```sh
node_modules/.bin/esbuild frontend/src/test/vaultNavigation.fixture.tsx --bundle --format=iife --outfile=.tmp/vault-render/fixture.js --loader:.jpg=file --define:process.env.NODE_ENV='"production"'
cp design/vault-navigation/review-shell.html .tmp/vault-render/index.html
swiftc -module-cache-path .tmp/vault-render/module-cache design/vault-navigation/capture-review.swift -o .tmp/vault-render/capture
.tmp/vault-render/capture "$PWD/.tmp/vault-render/index.html" "$PWD/.tmp/vault-render/folders-wide.png" 1440 1000
```

The fixture also chooses scenarios by shell filename (`search.html`, `moc.html`, `list.html`, `error.html`, `media-error.html`, `empty.html`, `loading.html`, `en.html`); copy the same review shell to those names. The capture helper's optional fifth argument sets reader scrollTop and optional sixth argument performs a fixture UI action. The helper reads local assets under the current worktree only and writes PNG plus DOM-metric JSON at the supplied output path. It does not read the installed app or personal Vault.

## Library delivery

Eight implementation/comparison PNGs are saved through the supported direct local-file route, including a new Search screenshot. Existing images were updated in place with version guards after the selected-state correction. Every response was `succeeded`, and local identity metadata was persisted. [Exact current identifiers, filenames and versions](library-delivery.json) are authoritative.

## Selected-state regression correction

Parent pixel inspection identified a real visual mismatch in the earlier wide comparison: Search retained its white surface while the folder tree was displayed, and the selected note lost its pink surface. This was not an intentional alternate screen. DOM state was correct (`vault-tab-folders` had `aria-selected=true`; the current note retained `is-selected`), but computed backgrounds in the WKWebView were stale: Search was white and Folders transparent. The shared button background transition retained the previous painted selection in this renderer. Earlier geometry checks did not catch that discrepancy.

Production CSS now disables transitions on Vault tabs and note/MOC/result rows so their surfaces change synchronously with the actual selected state. Search results also now expose `aria-current=page`, matching folder and MOC rows. No screenshot-only styling is injected.

The React regression switches all three tabs, checks visible content, follows notes, verifies current classes and checks narrow-list return. The read-only `review-selection-audit.js` checks actual computed colors and content visibility in WKWebView before allowing a capture. It rejects any inactive white tab, missing active white tab, wrong tree/form visibility or a current row without the approved pink surface and text color. Adjacent `.selection.json` files record these checks; Search, MOC, Folders and narrow-list checks all returned `problems: []`. The helper initially rejected the Search fixture because its result lacked `aria-current`; that production omission was then fixed. Final wide/narrow/MOC comparisons were regenerated and directly inspected, with the correct Folders/MOC white tab and pink selected note visible. Interactive media/source/link and error/loading screenshots were regenerated under the same gate.

Final follow-up verification: 65 test files / 496 tests, required runtime checks, lint, typecheck, build and whitespace checks passed. Native code was unchanged by this correction, so previous native results and the unresolved index timing limit above remain applicable. No packaged E2E or release build was added for this CSS/ARIA correction; actual rendered computed-style assertions cover its concrete risk.

## Authorized note-limit adjustment

The user explicitly raised the canonical/original note-read cap from 2 MiB to 10 MiB (10,485,760 bytes). `vault::NOTE_READ_LIMIT` now supplies the common `vault::read` and navigation metadata budgets, avoiding inconsistent readable flags between listing and opening. The raster image cap remains its separate unchanged 10 MiB constant. Translation selection, caching, scheduling and publication behavior are unchanged. English/Korean read-error hints and guides identify the note limit.

The earlier 2 MiB value was a conservative implementation guard: its code checked filesystem length before `fs::read_to_string`, and navigation used a bounded read. There was no browser-imposed 2 MiB standard or measured Markdown-rendering breakpoint behind that number. Boundary tests exercise native reads and listing at limit minus one byte, exactly the limit, and limit plus one byte; they do not measure UI rendering speed. Large-Markdown UI rendering performance is still unmeasured, so acceptance by the API is not a smooth-rendering performance guarantee.

Packaged E2E is skipped for this numeric read-budget and error-copy change: native application-command boundary tests directly exercise the accepted/rejected byte sizes and unchanged source files; frontend tests check the limit hint and existing state behavior. No release build or screenshot regeneration is needed for the unchanged layout. The earlier independent native index-timing limitation remains unresolved.

The 10 MiB follow-up passed its exact-byte native boundary test and all 15 application-command tests, plus lint, typecheck, build and whitespace validation. The first frontend run had 495 passes and an existing Task IME/Enter timing failure during concurrent compilation; without a Task change, a subsequent run passed all 496 tests and required runtime checks. The native boundary test accepted 10,485,759 and 10,485,760 bytes, rejected 10,485,761 bytes with the explicit limit message, and verified listing readability and unchanged source sizes. No large-Markdown UI rendering benchmark was run.

## Approved main integration

The user explicitly approved integrating the Vault implementation and 10 MiB follow-up into main and pushing normally. The approved source commits remain preserved by local backup ref `backup/vault-approved-e253feb`. Rebase onto remote main `9940d91` completed without conflicts; existing Compass changes remain present and notification/session branches are excluded.

Integration checks passed: 65 frontend test files / 498 tests plus required runtime checks, lint, typecheck, production build, 15 native application-command tests and whitespace validation. `cargo fmt --check` still fails across the existing Rust formatting baseline; an archived copy of remote main also fails the same check. No broad unrelated Rust cleanup was performed. The previous full-native index timing limitation and packaged E2E skip rationale remain applicable. This integration does not claim a completely green repository-wide native or formatting baseline.
