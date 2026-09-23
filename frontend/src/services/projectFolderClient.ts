import { invoke } from "@tauri-apps/api/core";

export const chooseProjectFolder = (): Promise<string | null> =>
  window.__TAURI_INTERNALS__
    ? invoke<string | null>("choose_project_folder")
    : Promise.resolve(null);
