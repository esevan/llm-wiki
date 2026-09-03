# First-run Vault setup

**English** | [한국어](first-run-vault-setup.ko.md)

On a new installation, LLM Wiki asks the user to choose an existing folder for the Markdown Vault
before the main application becomes interactive. The native operating-system folder picker owns
the choice; React never receives permission to submit an arbitrary filesystem path.

Closing the picker leaves setup incomplete and returns to the setup screen. It does not silently
create or adopt the default Documents folder. After a folder is selected, the native application:

1. verifies that the selection is an existing directory;
2. stores the canonical path in the local SQLite application settings;
3. restarts against the selected Vault; and
4. indexes its Markdown in the background with the bundled embedding model.

The SQLite workflow database remains in the platform application-data folder. Vault documents stay
as portable Markdown in the folder the user chose. Existing installations without an explicit
Vault setting retain the former `Documents/LLM Wiki Vault` location and do not see a migration
prompt. `LLM_WIKI_VAULT` remains an explicit development and test override.

If a previously selected directory is no longer available, the application returns to this setup
screen instead of creating a replacement directory at the stored path. This avoids silently
indexing an empty path when an external or synchronized Vault is disconnected.

Changing the Vault after onboarding is intentionally outside this first-run flow because switching
a live Vault also needs conflict, indexing, and in-flight job policy.
