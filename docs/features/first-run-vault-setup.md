# First-run Vault setup

**English** | [한국어](first-run-vault-setup.ko.md)

On a genuine new installation, LLM Wiki first presents a three-part welcome motion surface. It is a
separate transparent, borderless window sized to the current monitor: the centered application
window remains beneath a translucent animated layer so the introduction feels like part of the
desktop rather than another application dialog. The same React/CSS motion runs on macOS and
Windows, and operating-system reduced-motion preferences disable automatic scene changes and
decorative animation. The English and Korean versions use curated phrase-level line breaks so headings
remain balanced and Korean words are never split between lines.

The introduction appears once. **Skip intro** and **Choose Vault** both permanently complete it and
open the native operating-system folder picker; there is deliberately no replay setting or menu
item. React never receives permission to submit an arbitrary filesystem path.

Closing the picker leaves Vault setup incomplete and returns to the setup screen without replaying
the welcome motion. It does not silently create or adopt the default Documents folder. After a
folder is selected, the native application:

1. verifies that the selection is an existing directory;
2. stores the canonical path in `~/.llm-workbench/settings.json`;
3. restarts against the selected Vault; and
4. indexes its Markdown in the background with the bundled embedding model.

The SQLite workflow database remains in the platform application-data folder and no longer owns
application settings. Vault documents stay as portable Markdown in the folder the user chose.
Existing installations without an explicit
Vault setting retain the former `Documents/LLM Wiki Vault` location and do not see a migration
prompt or welcome motion. An interrupted welcome resumes on the next launch until the user skips
or completes it. `LLM_WIKI_VAULT` remains an explicit development and test override.

If a previously selected directory is no longer available, the application returns to this setup
screen instead of creating a replacement directory at the stored path. This avoids silently
indexing an empty path when an external or synchronized Vault is disconnected.

Changing the Vault after onboarding is intentionally outside this first-run flow because switching
a live Vault also needs conflict, indexing, and in-flight job policy.
