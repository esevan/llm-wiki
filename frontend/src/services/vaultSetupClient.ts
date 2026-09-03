import { invoke } from '@tauri-apps/api/core';

export interface VaultSetupStatus {
  required: boolean;
  path: string | null;
}

const ready: VaultSetupStatus = { required: false, path: null };

export const getVaultSetupStatus = (): Promise<VaultSetupStatus> =>
  window.__TAURI_INTERNALS__ ? invoke<VaultSetupStatus>('vault_setup_status') : Promise.resolve(ready);

export const chooseVault = (): Promise<boolean> => invoke<boolean>('choose_vault');
