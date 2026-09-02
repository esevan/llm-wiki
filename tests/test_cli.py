from pathlib import Path

from llm_wiki.cli import SERVICE_LABEL, launch_agent_definition


def test_launch_agent_is_loopback_service_with_project_venv_binary(tmp_path: Path) -> None:
    definition = launch_agent_definition(tmp_path / "project", tmp_path / "vault", tmp_path / "logs")
    assert definition["Label"] == SERVICE_LABEL
    assert definition["ProgramArguments"] == [
        str(tmp_path / "project" / ".venv" / "bin" / "llm-wiki"), "serve", "--vault", str(tmp_path / "vault"), "--no-browser"
    ]
    assert definition["RunAtLoad"] is True
    assert definition["KeepAlive"] == {"Crashed": True}
