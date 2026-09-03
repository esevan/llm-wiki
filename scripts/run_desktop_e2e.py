from __future__ import annotations

import json
import os
import socket
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "src-tauri" / "target" / "release" / "bundle" / "macos" / "LLM Wiki.app"
EXECUTABLE = APP / "Contents" / "MacOS" / "llm-wiki-desktop"


def available_port() -> int:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def main() -> None:
    if not EXECUTABLE.is_file():
        raise SystemExit("Build the desktop bundle with `npm run tauri:build` first.")
    with tempfile.TemporaryDirectory(dir=ROOT / ".tmp", prefix="desktop-e2e-") as directory:
        state = Path(directory)
        vault = state / "vault"
        vault.mkdir()
        (vault / "startup.md").write_text(
            "# Startup indexing\n\nThe bundled embedding model indexes this note before the UI starts.\n",
            encoding="utf-8",
        )
        result = state / "result.json"
        provider_port = available_port()
        provider = subprocess.Popen(
            [str(ROOT / ".venv/bin/python"), "tests/fakes/openai_server.py", "--port", str(provider_port)],
            cwd=ROOT,
        )
        for _ in range(100):
            try:
                with socket.create_connection(("127.0.0.1", provider_port), timeout=0.1):
                    break
            except OSError:
                if provider.poll() is not None:
                    raise SystemExit("The deterministic provider stopped during startup.")
                time.sleep(0.02)
        else:
            raise SystemExit("The deterministic provider did not become ready.")
        environment = {
            **os.environ,
            "LLM_WIKI_VAULT": str(vault),
            "LLM_WIKI_DB": str(state / "state.sqlite3"),
            "LLM_WIKI_E2E_RESULT": str(result),
            "LLM_WIKI_E2E_PROVIDER_URL": f"http://127.0.0.1:{provider_port}/v1",
            "LLM_WIKI_TEST_MODE": "1",
            "LLM_WIKI_TEST_API_KEY": "desktop-e2e-key",
        }
        process = subprocess.Popen([str(EXECUTABLE)], cwd=ROOT, env=environment)
        try:
            for launch in range(2):
                deadline = time.monotonic() + 180
                while not result.is_file() and process.poll() is None and time.monotonic() < deadline:
                    time.sleep(0.1)
                if not result.is_file():
                    started = result.with_suffix(".started").is_file()
                    progress = result.with_suffix(".progress")
                    completed_steps = json.loads(progress.read_text(encoding="utf-8")) if progress.is_file() else []
                    raise SystemExit(
                        "Desktop scenario did not report a result "
                        f"(launch={launch + 1}, exit={process.poll()}, webview_started={started}, "
                        f"completed_steps={completed_steps})."
                    )
                payload = json.loads(result.read_text(encoding="utf-8"))
                if payload["status"] != "relaunch":
                    break
                process.wait(timeout=10)
                environment["LLM_WIKI_E2E_RESTORE_CAPTURE"] = payload["capture"]
                environment["LLM_WIKI_E2E_RESTORE_STEPS"] = json.dumps(payload["steps"])
                result.unlink()
                process = subprocess.Popen([str(EXECUTABLE)], cwd=ROOT, env=environment)
            else:
                raise SystemExit("Desktop scenario requested more than one relaunch.")
            if payload["status"] != "passed":
                raise SystemExit(f"Desktop scenario failed: {payload}")
            print("desktop E2E passed")
            for step in payload["steps"]:
                print(f"- {step}")
        finally:
            if process.poll() is None:
                process.terminate()
            process.wait(timeout=10)
            if provider.poll() is None:
                provider.terminate()
            provider.wait(timeout=10)


if __name__ == "__main__":
    main()
