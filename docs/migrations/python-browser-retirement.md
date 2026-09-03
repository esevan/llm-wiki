# Python browser delivery retirement

The FastAPI browser delivery was intentionally retired after native command parity was verified.
Its final source and 196 passing characterization tests remain available in Git at `caef236`.

Removed runtime surfaces include `llm_wiki/`, `pyproject.toml`, `uv.lock`, the browser/API tests,
the Python benchmark entry point, the HTTP application adapter, and the LaunchAgent workflow.
Static UI sources moved to `frontend/`; generated assets now live only in ignored `dist/` output.

No Python behavior was silently discarded: the application behavior mapping and final native
coverage are retained in [react-tauri-report.md](react-tauri-report.md) and
[react-tauri-inventory.md](react-tauri-inventory.md). HTTP-only routing, CORS, and server lifecycle
assertions were removed because the corresponding product surface was removed.

To inspect the retired implementation without restoring it into the working tree:

```text
git show caef236:llm_wiki/web/app.py
git ls-tree -r --name-only caef236 llm_wiki tests
```

Reintroducing the browser delivery is a product decision and must not be done as a desktop
compatibility shim.
