from pathlib import Path

from llm_wiki import cli
from llm_wiki.cli import SERVICE_LABEL, launch_agent_definition


def test_launch_agent_is_loopback_service_with_project_venv_binary(tmp_path: Path) -> None:
    definition = launch_agent_definition(tmp_path / "project", tmp_path / "vault", tmp_path / "logs")
    assert definition["Label"] == SERVICE_LABEL
    assert definition["ProgramArguments"] == [
        str(tmp_path / "project" / ".venv" / "bin" / "llm-wiki"),
        "serve",
        "--vault",
        str(tmp_path / "vault"),
        "--no-browser",
    ]
    assert definition["RunAtLoad"] is True
    assert definition["KeepAlive"] == {"Crashed": True}


def test_desktop_boundary_starts_workers_inside_the_supervised_process(tmp_path: Path, monkeypatch) -> None:
    started: list[tuple[str, tuple[object, ...]]] = []

    class Thread:
        def __init__(self, *, target, args, name, daemon):
            assert daemon is True
            started.append((name, args))

        def start(self) -> None:
            return None

        def join(self, *, timeout: int) -> None:
            assert timeout == 5

    served: list[tuple[str, int]] = []
    monkeypatch.setattr(cli.threading, "Thread", Thread)
    monkeypatch.setattr(cli, "_configured_worker_count", lambda _db: 3)
    monkeypatch.setattr(cli, "create_app", lambda vault, db, fast_queue_client: (vault, db, fast_queue_client))
    monkeypatch.setattr(cli.uvicorn, "run", lambda _app, host, port: served.append((host, port)))

    cli._serve_desktop(tmp_path / "vault", tmp_path / "state.sqlite3", 9123)

    assert [name for name, _args in started] == ["llm-wiki-desktop-fast-worker", "llm-wiki-async-workers"]
    assert started[1][1][2] == 3
    assert started[1][1][3].is_set()
    assert served == [("127.0.0.1", 9123)]
