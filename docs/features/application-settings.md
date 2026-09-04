# Application settings storage

**English** | [한국어](application-settings.ko.md)

LLM Wiki stores non-secret application settings in one user-owned JSON file:

```text
~/.llm-workbench/settings.json
```

On Windows, `~` is the current user profile, so the path is
`%USERPROFILE%\.llm-workbench\settings.json`. The file contains the selected Vault path,
first-run state, explicit locale choice, provider endpoint and model routing, report language, and
background worker count. API keys are never written there; macOS Keychain or Windows Credential
Manager continues to own provider secrets.

The current settings document is version 2. `introCompleted` is written as `false` only for a
genuine new installation and becomes `true` when the introduction is skipped or completed. A
missing field means an installation predates the introduction, so upgrades do not unexpectedly
show first-install motion.

Settings writes use a process-wide lock and temporary-file replacement. On Unix, the directory is
restricted to mode `0700` and the file to `0600`. Workflow records, indexes, jobs, notifications,
and generated application state remain in the platform application-data SQLite database.

When upgrading from a version that stored settings in SQLite, the native startup path imports the
legacy Vault, locale, and provider values only if `settings.json` does not exist. Existing database
rows are left intact for rollback safety but are no longer read or updated after migration. New
databases do not create the old settings tables.

`LLM_WORKBENCH_HOME` may redirect the settings directory for isolated development and automated
tests. Production packages always use the current user's home directory unless that explicit
process-level override is supplied.
