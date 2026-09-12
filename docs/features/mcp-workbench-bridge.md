# Continue Task work from Codex and ChatGPT desktop

**English** | [한국어](mcp-workbench-bridge.ko.md)

An approved local MCP connection can read and propose changes to the same Task-centered work records as the desktop Workbench. It works through explicit connection scopes and topic membership; it does not expose the Vault, app database, or credentials as unrestricted files.

## Task proposals and review

A chat can capture a thought, propose a Task action, add a checkpoint, or propose a completion or Knowledge action. The proposal identifies its Task and expected revision. When Problem context is needed, it carries an exact Problem revision as optional provenance; a Task does not need a Problem parent.

The host presents the exact proposal for review. The user can accept, reject, edit, defer, or request another review where the available action permits it. A proposal never changes the Task merely because an AI generated it. Stale, cancelled, expired, and rejected proposals remain auditable without replacing current state.

## Limits and continuity

Workbench, in-app chat, and local MCP use the same application boundary, so accepted actions receive the same revision checks and persistence rules. Completing tracked chat work does not publish Knowledge. Knowledge drafting and publication remain separate explicit decisions.

The current bridge is local, stdio-based integration for supported desktop hosts. It does not promise ChatGPT web, remote MCP, automatic task selection, unrestricted cross-topic access, or automatic approval of AI proposals.

## Set up a local connection

Open **AI setup** and use the **Chat connections** section. Enter a connection name, choose only the needed grants, and optionally list allowed topic IDs; create the connection. Expand its **Granted access** details to review the exact grants and the local command shown there: `llm-wiki-desktop --mcp --connection <id>`. Use that command in a supported local desktop host's stdio MCP configuration. If the executable is not on PATH, use the installed application's full executable path. Keep the GUI running: the stdio bridge forwards to it and fails closed when it is unavailable. Manage topic membership from the same section, and use **Revoke** to disable a connection immediately.

See [Task-centered Workbench](conflict-gated-workflow.md) and the current historical context in [specification 012](../../specs/012-task-centered-workbench/spec.md).
