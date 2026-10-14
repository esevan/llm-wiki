# Application settings storage

**English** | [한국어](application-settings.ko.md)

LLM Wiki stores application settings, including the API key, in one user-owned JSON file:

```text
~/.llm-workbench/settings.json
```

On Windows, `~` is the current user profile, so the path is
`%USERPROFILE%\.llm-workbench\settings.json`. The file contains the selected Vault path,
first-run state, explicit locale choice, provider endpoint and model routing, report language, and
background worker count, and API key. The API key is never returned through the UI or MCP, and it
is never written to the Vault or application database. A blank API-key field preserves the stored
key while other provider settings are updated.

AI setup exposes a shared **AI request timeout (seconds)** for all provider HTTP requests, including streaming conversations and connection tests. It accepts whole seconds from 1 to 3600 and defaults to 180 when the field is absent. The JSON provider field is `requestTimeoutSeconds` (`request_timeout_seconds` in the configuration API). Saving changes the deadline for new requests without restarting; requests already running retain their deadline.

AI setup also shows the active Vault path. Change Vault location opens the native folder picker and
restarts the app with the selected folder. Regenerate embeddings scans the current Vault and rebuilds
semantic embeddings for its Markdown documents using the bundled model; it does not delete Vault files
or application records.

The current settings document is version 2. `introCompleted` is written as `false` only for a
genuine new installation and becomes `true` when the introduction is skipped or completed. A
missing field means an installation predates the introduction, so upgrades do not unexpectedly
show first-install motion.

A process-wide `SettingsStore` singleton owns all settings reads and writes. It loads each settings file once and serves immutable snapshots to the system. Updates serialize through its lock, replace the file atomically, and publish the new snapshot only after a successful write. Failed writes preserve the previous in-memory values. Isolated application homes keep separate snapshots. Manual edits to the JSON file require restarting the app. On Unix, the directory is
restricted to mode `0700` and the file to `0600`. Workflow records, indexes, jobs, notifications,
and generated application state remain in the platform application-data SQLite database.

When upgrading from a version that stored settings in SQLite, the native startup path imports the
legacy Vault, locale, and provider values only if `settings.json` does not exist. Existing database
rows are left intact for rollback safety but are no longer read or updated after migration. New
databases do not create the old settings tables.

`LLM_WORKBENCH_HOME` may redirect the settings directory for isolated development and automated
tests. Production packages always use the current user's home directory unless that explicit
process-level override is supplied.
