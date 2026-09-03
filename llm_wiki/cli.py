from __future__ import annotations

import argparse
import asyncio
import multiprocessing
import os
import plistlib
import sqlite3
import subprocess
import threading
import webbrowser
from pathlib import Path

import uvicorn
from platformdirs import user_data_path

from llm_wiki.services.fast_queue import FastQueueServer
from llm_wiki.services.job_runtime import run_async_workers
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.web.app import create_app

SERVICE_LABEL = "com.llm-wiki"


def _run_fast_worker(host: str, port: int) -> None:
    asyncio.run(FastQueueServer(host, port).serve())


def _run_async_worker(vault: Path, db_path: Path, count: int, stop_requested: threading.Event | None = None) -> None:
    async def supervise() -> None:
        stop = asyncio.Event()
        workers = asyncio.create_task(run_async_workers(vault, db_path, count, stop))
        if stop_requested is None:
            await workers
            return
        while not stop_requested.is_set():
            await asyncio.sleep(0.05)
        stop.set()
        await workers

    asyncio.run(supervise())


def _serve_web(vault: Path, db_path: Path, port: int = 8765) -> None:
    uvicorn.run(create_app(vault, db_path), host="127.0.0.1", port=port)


def _configured_worker_count(db_path: Path) -> int:
    connection = sqlite3.connect(db_path)
    try:
        return ProviderSettings(connection).async_worker_count()
    finally:
        connection.close()


def launch_agent_definition(project_root: Path, vault: Path, log_dir: Path) -> dict[str, object]:
    """A per-user macOS service; no network listener beyond loopback is configured."""
    return {
        "Label": SERVICE_LABEL,
        "ProgramArguments": [
            str(project_root / ".venv" / "bin" / "llm-wiki"),
            "serve",
            "--vault",
            str(vault),
            "--no-browser",
        ],
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
    web = sub.add_parser("web", help="Run only the HTTP process")
    web.add_argument("--vault", type=Path, required=True)
    fast = sub.add_parser("fast-worker", help="Run the single ephemeral AI throttle")
    fast.add_argument("--port", type=int, default=8766)
    asynchronous = sub.add_parser("async-worker", help="Run durable AI workers")
    asynchronous.add_argument("--vault", type=Path, required=True)
    asynchronous.add_argument("--count", type=int, default=2)
    install = sub.add_parser("install-service", help="Start LLM Wiki at macOS login using launchd")
    install.add_argument("--vault", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "install-service":
        install_service(args.vault)
        return
    data_dir = user_data_path("LLM Wiki", appauthor=False)
    data_dir.mkdir(parents=True, exist_ok=True)
    db_path = data_dir / "llm-wiki.sqlite3"
    if args.command == "fast-worker":
        _run_fast_worker("127.0.0.1", args.port)
        return
    if args.command == "async-worker":
        _run_async_worker(args.vault, db_path, args.count)
        return
    if args.command == "web":
        _serve_web(args.vault, db_path)
        return
    if not args.no_browser:
        webbrowser.open("http://127.0.0.1:8765")
    context = multiprocessing.get_context("spawn")
    fast_process = context.Process(target=_run_fast_worker, args=("127.0.0.1", 8766), name="llm-wiki-fast-worker")
    worker_count = _configured_worker_count(db_path)
    async_processes = [
        context.Process(
            target=_run_async_worker,
            args=(args.vault, db_path, 1),
            name=f"llm-wiki-async-worker-{index + 1}",
        )
        for index in range(worker_count)
    ]
    processes = [fast_process, *async_processes]
    for process in processes:
        process.start()
    try:
        _serve_web(args.vault, db_path)
    finally:
        for process in processes:
            process.terminate()
        for process in processes:
            process.join(timeout=5)
