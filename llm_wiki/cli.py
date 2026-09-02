from __future__ import annotations

import argparse
import os
import plistlib
import subprocess
import webbrowser
from pathlib import Path

import uvicorn
from platformdirs import user_data_path

from llm_wiki.api.app import create_app


SERVICE_LABEL = "com.llm-wiki"


def launch_agent_definition(project_root: Path, vault: Path, log_dir: Path) -> dict[str, object]:
    """A per-user macOS service; no network listener beyond loopback is configured."""
    return {
        "Label": SERVICE_LABEL,
        "ProgramArguments": [str(project_root / ".venv" / "bin" / "llm-wiki"), "serve", "--vault", str(vault), "--no-browser"],
        "WorkingDirectory": str(project_root),
        "RunAtLoad": True,
        "KeepAlive": {"Crashed": True},
        "ProcessType": "Background",
        "StandardOutPath": str(log_dir / "llm-wiki.out.log"),
        "StandardErrorPath": str(log_dir / "llm-wiki.err.log"),
    }


def install_service(vault: Path) -> None:
    if os.name != "posix" or not Path("/System/Library").exists():
        raise SystemExit("The local service installer currently supports macOS launchd only.")
    project_root = Path(__file__).resolve().parents[1]
    executable = project_root / ".venv" / "bin" / "llm-wiki"
    if not executable.exists():
        raise SystemExit("Run `uv run llm-wiki --help` once from the project to create its environment.")
    vault = vault.expanduser().resolve()
    if not vault.is_dir():
        raise SystemExit(f"Vault directory does not exist: {vault}")
    agents = Path.home() / "Library" / "LaunchAgents"
    logs = user_data_path("LLM Wiki", appauthor=False) / "logs"
    agents.mkdir(parents=True, exist_ok=True)
    logs.mkdir(parents=True, exist_ok=True)
    plist_path = agents / f"{SERVICE_LABEL}.plist"
    with plist_path.open("wb") as handle:
        plistlib.dump(launch_agent_definition(project_root, vault, logs), handle, sort_keys=False)
    domain = f"gui/{os.getuid()}"
    subprocess.run(["launchctl", "bootout", domain, str(plist_path)], check=False, capture_output=True)
    subprocess.run(["launchctl", "bootstrap", domain, str(plist_path)], check=True)
    print(f"LLM Wiki now starts at login: http://127.0.0.1:8765\nService file: {plist_path}")


def main() -> None:
    parser = argparse.ArgumentParser(prog="llm-wiki")
    sub = parser.add_subparsers(dest="command", required=True)
    serve = sub.add_parser("serve")
    serve.add_argument("--vault", type=Path, required=True)
    serve.add_argument("--no-browser", action="store_true")
    install = sub.add_parser("install-service", help="Start LLM Wiki at macOS login using launchd")
    install.add_argument("--vault", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "install-service":
        install_service(args.vault)
        return
    data_dir = user_data_path("LLM Wiki", appauthor=False)
    data_dir.mkdir(parents=True, exist_ok=True)
    if not args.no_browser:
        webbrowser.open("http://127.0.0.1:8765")
    uvicorn.run(create_app(args.vault, data_dir / "llm-wiki.sqlite3"), host="127.0.0.1", port=8765)
