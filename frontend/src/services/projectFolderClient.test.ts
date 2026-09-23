import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { chooseProjectFolder } from "./projectFolderClient";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("project folder picker", () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it("returns the selected folder without changing Vault settings", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {} });
    vi.mocked(invoke).mockResolvedValue("/workspace/project");

    await expect(chooseProjectFolder()).resolves.toBe("/workspace/project");
    expect(invoke).toHaveBeenCalledWith("choose_project_folder");
  });
});
