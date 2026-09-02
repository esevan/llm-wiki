# Quickstart: Validate Task-Level AI Model Routing

1. Open **AI setup**, enter the endpoint, Default model, and Advanced model.
2. Expand **Advanced options** and verify discussion and refinement are selected by default.
3. Change one task selection, save, leave the view, and return to verify it persisted.
4. Clear the Advanced model while keeping an advanced task selected; trigger that task and verify that it uses the Default model rather than failing for a missing Advanced model.
5. Run `uv run pytest -q`, the browser-script syntax check, and `git diff --check`.
