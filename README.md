# LLM Wiki

**English** | [한국어](README.ko.md)

> **You talk. The work organizes itself.**

LLM Wiki is an AI-centered, local-first workbench that turns conversation into organized work,
keeps enough context to resume, and publishes only completed outcomes as portable knowledge.

![The current LLM Wiki Workbench highlights a lightweight Capture entry and Solutions already in progress](docs/features/images/02-workbench.png)

## Product Spirit

LLM Wiki is designed from six non-negotiable principles:

1. **You talk. The work organizes itself.** Conversation and Refinement do the structuring.
2. **Reduce cognitive load.** Capture stays light; current work receives the strongest emphasis.
3. **Resume where you left off.** Work Log screenshots, preserved Refinement context, and Knowledge-backed conflict review make work resumable.
4. **Organize around problems, not tasks.** The durable workflow is **Capture → Problem → Solution**. Execution stays inside the Solution Work Log and validation checklist.
5. **Private process, portable knowledge.** Drafts and working context remain local; only a human-approved completed result becomes Markdown Knowledge.
6. **Understand the work, never score the worker.** Signals explain evidence, risk, and direction—not individual productivity.

See [Product Spirit in the product](docs/product-spirit.md) for the design implications and visible product evidence behind each principle.

## What the product does

| Need | LLM Wiki response |
| --- | --- |
| Get a thought out quickly | Capture accepts a lightweight starting thought without demanding structure. |
| Understand the real problem | AI-guided Refinement preserves context and proposes a reviewable Problem. |
| Choose a way forward | A human-approved Problem can produce a Solution; Knowledge-backed conflict review must be clear before work starts. |
| Resume active work | In Progress Solutions are highlighted; Work Log stores text, screenshots, comments, and validation checks. |
| Complete with evidence | AI reviews recorded evidence, while the human retains the completion decision. |
| Reuse the result | Only completed work becomes an Obsidian-compatible Playbook and searchable Knowledge. |
| Stay responsive during AI work | One hidden Fast Queue throttles interaction; durable work remains visible and recoverable in the background Queue. |

There is no Task stage. LLM Wiki keeps execution attached to the Solution so work never loses the
Problem that explains why it exists.

## Quick start

Requirements: Node.js 22 LTS, the stable Rust toolchain, and the platform prerequisites from the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```text
npm ci
LLM_WIKI_VAULT=/path/to/your-vault npm run tauri -- dev
```

Configure your OpenAI-compatible endpoint and models in **AI setup**. AI is a required product
capability; local search and manual controls are resilience fallbacks that preserve private process
and human authority during provider failure.

Non-secret settings are stored in `~/.llm-workbench/settings.json`. API keys are stored in macOS
Keychain or Windows Credential Manager, never in the Vault, settings file, or app database. The
desktop process opens its SQLite database and selected Vault directly through Rust domain commands:

```text
npm run tauri:build
```

The desktop build downloads a pinned, checksum-verified multilingual MiniLM ONNX model into the
ignored build-resource cache and packages it in the application. A released app performs semantic
indexing and search locally and never downloads an embedding model at startup. Re-run
`npm run prepare:embedding` to verify or restore the cached build assets.

Nunito, DM Mono, and variable Noto Sans KR are also copied into the application assets during every
production build. The packaged UI therefore renders Korean and English without a web-font request.
macOS builds produce an `.app`; Windows builds produce MSI and NSIS installers. A reproducible
Windows agent workflow is documented in [Windows packaging and installation](docs/windows-packaging.md).

The release `.app` contains no Python runtime, sidecar, or internal web server. A new installation
opens a native folder picker before the Workbench becomes interactive and remembers the selected
Markdown Vault. Existing installations retain `Documents/LLM Wiki Vault`; `LLM_WIKI_VAULT` remains
a development override. React invokes separate workflow, Vault, settings, jobs, and system
commands; chat chunks and cancellation use a native Tauri channel. Network sockets are opened only
for an explicitly configured external AI provider. See
[First-run Vault setup](docs/features/first-run-vault-setup.md).
See [Application settings storage](docs/features/application-settings.md) for the file schema,
legacy SQLite migration, and platform paths.
Workflow data uses ordered, transactional SQLite schema migrations and rejects databases created by
a newer incompatible app. See [SQLite schema migrations](docs/database-migrations.md).
The former Python/FastAPI browser delivery was retired after native command parity was verified.
Its final implementation remains available only in Git history at `caef236`.

See [Background AI Queue](docs/features/background-ai-queue.md) for worker roles, recovery,
task-specific results, and notification behavior.

### Korean and English

Use the global language control to switch the interface between Korean and English without
reloading or losing the current view and input. The explicit choice is retained for later sessions;
before a choice is saved, a Korean environment uses Korean and other environments use English.

New, reviewed AI-generated Problems and Solutions keep Korean and English stored versions, while
existing records and Vault files remain unchanged and fall back to their stored original. Live AI
content uses the language active when its request begins. App-managed Knowledge remains
English-canonical Markdown; Korean reading is generated on request without replacing that portable
source. See [Korean and English localization](docs/features/bilingual-localization.md).

### AI model routing

AI Setup has two model fields: a **Default model** for routine work and an **Advanced model** for
quality-sensitive work. Expand **Advanced options** to choose the model tier for each AI task.
Unchecked tasks use the Default model; enabled tasks use the Advanced model, and safely fall back
to the Default model when no Advanced model is configured.

The initial routing uses the Advanced model for discussions and refinement, drafting Problems and
Solutions, conflict review, image summaries, completion review, and completion reports. Workbench
organization, completed-Solution discussion, and Problem enrichment use the Default model by
default. Every choice remains user-configurable in Advanced options.

## Feature guides

- [Product Spirit and product decisions](docs/product-spirit.md)
- [Visual feature tour](docs/features/visual-guide.md)
- [Fast vault search](docs/features/fast-vault-search.md)
- [Problem-centered Workbench](docs/features/conflict-gated-workflow.md)
- [Completion, Knowledge, and archive](docs/features/completion-writeback-archive.md)
- [Compass](docs/features/direction-dashboard.md)
- [Context-preserving Refinement Preview](docs/features/refinement-preview-status.md)
- [Korean and English localization](docs/features/bilingual-localization.md)

## Development

Create isolated task branches with the shared-cache worktree helper described in
[Fast task worktrees](docs/worktree-workflow.md). New worktrees reuse Node dependencies, compiled
Rust dependencies, and verified embedding assets instead of rebuilding them from zero.

Every specification, plan, implementation, and review must pass the Product Spirit Review Gate in
the [project constitution](.specify/memory/constitution.md).

Application layers, dependency direction, and native task boundaries are documented in the
[architecture guide](docs/architecture.md).

```text
npm test
npm run typecheck
npm run lint
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri:build
npm run test:desktop
```

Spec Kit artifacts live under [specs/](specs/).
