import type { ApplicationClient } from '../types/application';
import { HttpApplicationClient } from './httpApplicationClient';
import { TauriApplicationClient } from './tauriApplicationClient';

export const createApplicationClient = (): ApplicationClient =>
  window.__TAURI_INTERNALS__ ? new TauriApplicationClient() : new HttpApplicationClient();
