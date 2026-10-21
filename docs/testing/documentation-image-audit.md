# Documentation image audit — 2026-10-21

All current screenshot references refreshed against `584ae3c`; explicitly historical images are preserved. This is **full App synthetic WKWebView evidence**, not installed-native-app evidence.

## Baseline and scope

- Dedicated worktree: `.worktrees/refresh-document-images`; branch `docs/refresh-document-images`.
- Initial read-only remote main check: `e2f07be5c8de12a246920ff1f9f865f9d4f036f5`. Initial local prerequisites were `44c7cd9` then `0242a71`.
- After final integration, only the documentation commit was rebased onto `584ae3c6a44042af41cc16c27d757bef87105c54`; prerequisite changes were not cherry-picked again.
- README, English/Korean guides, all SpecKit Markdown and image assets were inventoried. The initial baseline had 36 image references and 27 assets.
- Task tab-entry and Knowledge chat tests remain present; migration 25 and the 29-entry prompt registry are unchanged.
- No main/remote writes, merges, pushes, installed app changes, signing-session 2549 changes, user security changes or original DB access.

## Refresh and preservation decisions

| Screen/assets | Result |
| --- | --- |
| Workbench / Search / Compass / Queue / AI settings | Both locales recaptured from final source; exact PNG SHA-256 matches all ten earlier visually reviewed captures. |
| Task / preview | Both locales refreshed; Work/Sessions/Refine/Review tabs, preview editing location and removed duplicate Task definition represented. |
| Knowledge / comparison / applied private v2 | Both locales refreshed through actual rendered controls and synthetic revision responses; publication remains separate. |
| `task-review*.png` | Preserved and explicitly labeled historical in both guides: `ConflictReviewPanel` has no production App mount at this source revision. Do not stage it as an available Task screen. |
| Other previous PNGs and `current-*` JPEGs | Original bytes and native manifests preserved as dated history; current guides link new `integrated-*` assets. |
| Diagrams / SpecKit | No rendered diagram assets or Mermaid blocks found in the scanned Markdown. SpecKit016 `shot.png` is a JSON contract example, not an image link. No diagrams or historical specs were overwritten. |

The final set contains 20 compact JPEGs. Current images are reused across guides rather than copied. No image generation was used.

## Rendered image references

| Document:line | Alt | Target |
| --- | --- | --- |
| `README.md:12` | The current Workbench shows lightweight Capture and Task entry with saved work | `docs/features/images/integrated-workbench.en.jpg` |
| `README.ko.md:12` | 가벼운 Capture·Task 입력과 저장된 작업을 보여 주는 최신 Workbench | `docs/features/images/integrated-workbench.ko.jpg` |
| `docs/features/background-ai-queue.ko.md:24` | 지속 작업의 목적, 대상, 상태, 복구 동작을 읽을 수 있게 보여주는 백그라운드 Queue | `images/integrated-queue.ko.jpg` |
| `docs/features/background-ai-queue.ko.md:76` | 합성 endpoint와 model routing을 표시하며 API key 입력은 비어 있는 AI 설정 | `images/integrated-settings.ko.jpg` |
| `docs/features/background-ai-queue.md:25` | Background Queue shows durable work by readable purpose, target, status, and recovery action | `images/integrated-queue.en.jpg` |
| `docs/features/background-ai-queue.md:82` | AI setup with synthetic endpoint and model routing; the API key field is empty | `images/integrated-settings.en.jpg` |
| `docs/features/bilingual-localization.ko.md:8` | 선택한 시스템 언어로 표시하면서 작성한 콘텐츠는 원문으로 보존하는 Workbench | `images/integrated-workbench.ko.jpg` |
| `docs/features/bilingual-localization.md:8` | The Workbench uses the selected system language while preserving authored content | `images/integrated-workbench.en.jpg` |
| `docs/features/conflict-resolution-workflow.ko.md:7` | 근거 부족·재시도·시도 이력을 보여 주는 Task 충돌 검토 | `images/task-review.png` |
| `docs/features/conflict-resolution-workflow.md:7` | Task conflict review with cited findings, a Retry review action, and attempt history | `images/task-review.en.png` |
| `docs/features/direction-dashboard.ko.md:7` | 활동을 사람 점수로 바꾸지 않고 방향을 기록하는 Compass | `images/integrated-compass.ko.jpg` |
| `docs/features/direction-dashboard.md:7` | Compass records direction without turning activity into a worker score | `images/integrated-compass.en.jpg` |
| `docs/features/fast-vault-search.ko.md:7` | Vault 상대 경로와 일치한 노트 맥락을 보여주는 검색 결과 | `images/integrated-search.ko.jpg` |
| `docs/features/fast-vault-search.md:7` | Search results show vault-relative paths and matching note context | `images/integrated-search.en.jpg` |
| `docs/features/knowledge-distillation.ko.md:5` | Knowledge 수정 제안 비교와 넓어진 채팅 영역 | `images/integrated-knowledge-compare.ko.jpg` |
| `docs/features/knowledge-distillation.md:5` | Knowledge revision comparison with the expanded chat rail | `images/integrated-knowledge-compare.en.jpg` |
| `docs/features/lineage-knowledge-layer.ko.md:7` | 작업 근거와 출처를 함께 보존하는 Task 상세 | `images/integrated-task.ko.jpg` |
| `docs/features/lineage-knowledge-layer.md:7` | Task detail keeps work evidence and provenance together | `images/integrated-task.en.jpg` |
| `docs/features/reference-aware-workbench.ko.md:5` | 중복 정의 없이 작업 초안과 대화를 보여주는 다듬기 탭 | `images/integrated-preview.ko.jpg` |
| `docs/features/reference-aware-workbench.md:5` | Task Refine with the work preview and conversation, without duplicated definitions | `images/integrated-preview.en.jpg` |
| `docs/features/vault-conflict-evidence.ko.md:7` | 근거 부족·재시도·시도 이력을 보여 주는 Task 충돌 검토 | `images/task-review.png` |
| `docs/features/vault-conflict-evidence.ko.md:14` | 백그라운드 작업 실패와 복구 동작을 보여 주는 Queue | `images/integrated-queue.ko.jpg` |
| `docs/features/vault-conflict-evidence.md:7` | Task conflict review with cited findings, a Retry review action, and attempt history | `images/task-review.en.png` |
| `docs/features/vault-conflict-evidence.md:14` | Queue shows a background job failure and recovery action | `images/integrated-queue.en.jpg` |
| `docs/features/visual-guide.ko.md:16` | 합성 진행 중·완료·보류 Task가 있는 전체 App Workbench | `images/integrated-workbench.ko.jpg` |
| `docs/features/visual-guide.ko.md:29` | 작업 초안과 넓어진 대화 영역이 함께 있는 Task 다듬기 탭 | `images/integrated-preview.ko.jpg` |
| `docs/features/visual-guide.ko.md:47` | 합성 작업 기록이 저장된 Task 작업 탭 | `images/integrated-task.ko.jpg` |
| `docs/features/visual-guide.ko.md:64` | 비공개 Knowledge 초안과 넓어진 수정 채팅 | `images/integrated-knowledge.ko.jpg` |
| `docs/features/visual-guide.ko.md:69` | 변경 문장과 비공개 적용 동작이 표시된 Knowledge 수정 제안 | `images/integrated-knowledge-compare.ko.jpg` |
| `docs/features/visual-guide.ko.md:71` | 합성 수정 제안을 적용한 새 비공개 Knowledge 버전 | `images/integrated-knowledge-applied.ko.jpg` |
| `docs/features/visual-guide.md:18` | Full App Workbench with synthetic active, completed and held Tasks | `images/integrated-workbench.en.jpg` |
| `docs/features/visual-guide.md:32` | Task Refine tab with the work preview and expanded conversation rail | `images/integrated-preview.en.jpg` |
| `docs/features/visual-guide.md:50` | Task Work tab with a synthetic saved Work Log entry | `images/integrated-task.en.jpg` |
| `docs/features/visual-guide.md:67` | Private Knowledge draft beside the expanded revision chat | `images/integrated-knowledge.en.jpg` |
| `docs/features/visual-guide.md:72` | Knowledge revision proposal with its changed text and private apply action | `images/integrated-knowledge-compare.en.jpg` |
| `docs/features/visual-guide.md:74` | New private Knowledge revision after applying the synthetic proposal | `images/integrated-knowledge-applied.en.jpg` |
| `docs/product-spirit.ko.md:25` | Capture는 가볍게 유지하고 현재 Task를 바로 이어서 할 수 있게 하는 Workbench | `features/images/integrated-workbench.ko.jpg` |
| `docs/product-spirit.ko.md:39` | 최신 Work Log와 검증 맥락을 보존하는 Task 상세 | `features/images/integrated-task.ko.jpg` |
| `docs/product-spirit.ko.md:41` | 작업 초안과 대화를 함께 보여주는 Task 다듬기 | `features/images/integrated-preview.ko.jpg` |
| `docs/product-spirit.ko.md:61` | 별도 발행 승인 전 비공개로 유지되는 수정된 Knowledge 초안 | `features/images/integrated-knowledge-applied.ko.jpg` |
| `docs/product-spirit.md:27` | The Workbench keeps Capture light and makes current Tasks easy to resume | `features/images/integrated-workbench.en.jpg` |
| `docs/product-spirit.md:41` | A Task detail preserves the latest Work Log and validation context | `features/images/integrated-task.en.jpg` |
| `docs/product-spirit.md:43` | Task Refine keeps the work preview beside the conversation | `features/images/integrated-preview.en.jpg` |
| `docs/product-spirit.md:63` | A corrected Knowledge draft remains private until separate publication approval | `features/images/integrated-knowledge-applied.en.jpg` |

## Evidence manifest image references

These JSON entries describe historical captures, not currently rendered guide images. Their image names resolve under `docs/features/images/`; preserve the records and hashes instead of relabeling them as fresh captures.

| Manifest | Image references | Decision |
| --- | --- | --- |
| `docs/testing/evidence/capture-entry-layout.json` | `workbench-tasks.png`, `workbench-tasks.en.png` | Historical record preserved |
| `docs/testing/evidence/current-interface.json` | `current-workbench.en.jpg`, `current-workbench.ko.jpg`, `current-refinement.en.jpg`, `current-refinement.ko.jpg`, `current-task.en.jpg`, `current-task.ko.jpg`, `current-knowledge.en.jpg`, `current-knowledge.ko.jpg` | Historical record preserved |
| `docs/testing/evidence/docs-language-consistency.json` | `ai-settings.en.png`, `compass.en.png`, `completion-knowledge.en.png`, `queue-recovery.en.png`, `refinement.en.png`, `task-detail.en.png`, `task-review.en.png`, `vault-search.en.png`, `workbench-tasks.en.png` | Historical record preserved |
| `docs/testing/evidence/ui-ux-improvements.json` | `task-review.png`, `vault-search.png`, `workbench-tasks.png`, `task-detail.png`, `refinement.png`, `completion-knowledge.png`, `ai-settings.png`, `compass.png`, `queue-recovery.png` | Historical record preserved |

The historical manifests contain 28 image-name references. The new `integrated-interface-images.json` records all 20 current images separately.

Additional non-rendered mentions: SpecKit016 `shot.png` is a request payload example. `docs/testing/knowledge-revision-chat.md` describes five prior task-local screenshot filenames (`knowledge-revision-ko-wide.jpg`, `knowledge-revision-ko-compare.jpg`, `knowledge-revision-ko-applied.jpg`, `knowledge-revision-en-900.jpg`, `knowledge-revision-en-780.jpg`), not repository links. That historical test report is preserved; current guide captures are recorded in the new manifest.

## Asset inventory

| Asset | Bytes | SHA-256 prefix | Status |
| --- | ---: | --- | --- |
| `.gitkeep` | 1 | `01ba4719c80b6fe9` | Preserved historical bytes |
| `ai-settings.en.png` | 190115 | `0dd0bfd903675046` | Preserved historical bytes |
| `ai-settings.png` | 82658 | `6e456ff60ef9b969` | Preserved historical bytes |
| `compass.en.png` | 178473 | `5bc1cba75404973c` | Preserved historical bytes |
| `compass.png` | 71863 | `84508017c2915e03` | Preserved historical bytes |
| `completion-knowledge.en.png` | 300618 | `fd488a45beecbe84` | Preserved historical bytes |
| `completion-knowledge.png` | 111434 | `865ecea9c224cddb` | Preserved historical bytes |
| `current-knowledge.en.jpg` | 114603 | `88317853593959a7` | Preserved historical bytes |
| `current-knowledge.ko.jpg` | 111206 | `699a3e2d138e4b6a` | Preserved historical bytes |
| `current-refinement.en.jpg` | 103599 | `09ea30fdbb995403` | Preserved historical bytes |
| `current-refinement.ko.jpg` | 92216 | `0bd95dbb12d9204b` | Preserved historical bytes |
| `current-task.en.jpg` | 106319 | `eea8d0a0766a32cd` | Preserved historical bytes |
| `current-task.ko.jpg` | 93160 | `94ab49f6501985bd` | Preserved historical bytes |
| `current-workbench.en.jpg` | 88537 | `9e7c036adf65ef5e` | Preserved historical bytes |
| `current-workbench.ko.jpg` | 93951 | `b6faeee13a9e2142` | Preserved historical bytes |
| `integrated-compass.en.jpg` | 89175 | `6a699d2d84751e52` | Refreshed synthetic full App |
| `integrated-compass.ko.jpg` | 81831 | `d25eb1a96eb88e96` | Refreshed synthetic full App |
| `integrated-knowledge-applied.en.jpg` | 147915 | `f40ee0081bc08334` | Refreshed synthetic full App |
| `integrated-knowledge-applied.ko.jpg` | 153419 | `544c8e9cfa30d9c5` | Refreshed synthetic full App |
| `integrated-knowledge-compare.en.jpg` | 198381 | `922b27f0f01d861c` | Refreshed synthetic full App |
| `integrated-knowledge-compare.ko.jpg` | 198805 | `19f6fe2fdb39de18` | Refreshed synthetic full App |
| `integrated-knowledge.en.jpg` | 162216 | `091dfa185db31ef4` | Refreshed synthetic full App |
| `integrated-knowledge.ko.jpg` | 157574 | `d98a430dc9e4d002` | Refreshed synthetic full App |
| `integrated-preview.en.jpg` | 141979 | `c36f5a2ba7052375` | Refreshed synthetic full App |
| `integrated-preview.ko.jpg` | 132089 | `d07246623225259b` | Refreshed synthetic full App |
| `integrated-queue.en.jpg` | 114332 | `2be36b89c5428c84` | Refreshed synthetic full App |
| `integrated-queue.ko.jpg` | 106414 | `1229345968cde673` | Refreshed synthetic full App |
| `integrated-search.en.jpg` | 66727 | `b2190c64ebc6eedc` | Refreshed synthetic full App |
| `integrated-search.ko.jpg` | 63354 | `a0d5bea464421987` | Refreshed synthetic full App |
| `integrated-settings.en.jpg` | 113785 | `fa40210955261778` | Refreshed synthetic full App |
| `integrated-settings.ko.jpg` | 110434 | `eeb367163cc82930` | Refreshed synthetic full App |
| `integrated-task.en.jpg` | 108324 | `388c470b2abde46c` | Refreshed synthetic full App |
| `integrated-task.ko.jpg` | 104960 | `27cf639f364fa1d8` | Refreshed synthetic full App |
| `integrated-workbench.en.jpg` | 96576 | `5244e7d54cacce2a` | Refreshed synthetic full App |
| `integrated-workbench.ko.jpg` | 91131 | `50a32807f2fbe2e2` | Refreshed synthetic full App |
| `queue-recovery.en.png` | 260304 | `a32ffb89cb272238` | Preserved historical bytes |
| `queue-recovery.png` | 95649 | `da0493e6801c61f8` | Preserved historical bytes |
| `refinement.en.png` | 291699 | `30167467727a596a` | Preserved historical bytes |
| `refinement.png` | 108051 | `89a2fe3a3b23d7e5` | Preserved historical bytes |
| `task-detail.en.png` | 267415 | `666dc332c2e073c3` | Preserved historical bytes |
| `task-detail.png` | 106601 | `16485e7a3d07a6e2` | Preserved historical bytes |
| `task-review.en.png` | 320122 | `d0186cc6b6874ae5` | Preserved historical bytes |
| `task-review.png` | 103916 | `1e32d701d390eb54` | Preserved historical bytes |
| `vault-search.en.png` | 162256 | `ab99ddb6358ee998` | Preserved historical bytes |
| `vault-search.png` | 64200 | `a28551781d7421b3` | Preserved historical bytes |
| `workbench-tasks.en.png` | 196180 | `1c7c10ca66627827` | Preserved historical bytes |
| `workbench-tasks.png` | 178987 | `8419d2a3559912a0` | Preserved historical bytes |

## Reproduction

1. Use this worktree at source `584ae3c`; existing shared dependencies and copied `dist/assets/fonts.css` are sufficient. Do not run release builds, install apps, touch signing or open user storage.
2. Run `node docs/testing/image-capture/server.mjs` from the worktree. It serves only a loopback Vite fixture with the real App, shared styles/fonts, production runtime and in-memory synthetic responses.
3. Run `swift docs/testing/image-capture/render.swift 'http://127.0.0.1:5193/?lang=en&view=preview' .tmp/preview-en.png 1440`. Use `lang=ko` for Korean. Views: `workbench`, `search`, `compass`, `queue`, `ai-setup`, `task`, `preview`, `knowledge`, `knowledge-compare`, `knowledge-applied`.
4. The default height is 900; comparison captures use `1920 1080` as the final two arguments. Extra 1920/2560 width probes remain ignored temporary PNGs.
5. The fixture opens real controls, sends a synthetic revision request and clicks private apply. It never publishes. WKWebView uses `.nonPersistent()` and does not read the locked desktop. Existing Swift renderer and App fixture were reused, not generated UI imitations.
6. Inspect pixels, locale, font readiness, content, panel dimensions and overflow. Convert with `sips -s format jpeg -s formatOptions 80`; update relative links, alt and captions together. Stop the Vite process afterward.

## Verification and limits

- `npm test` passed on the final source, including all five mounted publication App cases. `git diff --check` passed. No native tests or packaged desktop E2E were needed for documentation-only changes.
- Measured preview and Knowledge chat widths match: 429.4375px at 1440px, 583.03125px at 1920px, 720px at 2560px. Duplicate `.task-definition-read` count is zero; preview editing UI remains in production source.
- All captured screens report intended locale, `fonts: loaded`, `applicationReady: true`, and no horizontal document overflow. All 20 tracked screens were visually reviewed; the first ten final-source PNGs exactly match their inspected predecessors.
- Two existing full-runtime startup errors reference absent `.workbench-context` event listeners in workbench/conflicts scripts. They are not suppressed or fixed here. Do not treat screenshots as proof of all runtime behavior.
- The browser file chooser follows host language and retains English native labels in Korean screenshots; product chrome and authored synthetic content match the document locale.
- Real model quality, original data, persistence, native focus/IME, file publication and indexing are not validated by synthetic capture. The locked native desktop was not accessed.
- Full hashes, image dimensions and renderer limits: [capture manifest](evidence/integrated-interface-images.json).
- Final local Markdown link check: 505 links, 0 missing; locale mismatches for current image links: 0. Duplicate asset byte sets: 0.
- New image bytes: 2,439,421; largest image: 198,805 bytes.

## Representative Library images

- Final Knowledge proposal comparison (Korean): `libfile_a20812a572808191b9fcbf099bea87d0`.
- Workbench (Korean): `libfile_f17310a0174c8191ae48b3bafe647181`; final source recapture has identical PNG pixels.

No screenshot work remains pending. Historical exclusions and native/real-data limits are recorded above.
