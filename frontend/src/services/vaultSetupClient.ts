import { invoke } from "@tauri-apps/api/core";

export interface VaultSetupStatus {
  required: boolean;
  path: string | null;
  introRequired: boolean;
}

const ready: VaultSetupStatus = {
  required: false,
  path: null,
  introRequired: false,
};

export const getVaultSetupStatus = (): Promise<VaultSetupStatus> =>
  window.__TAURI_INTERNALS__
    ? invoke<VaultSetupStatus>("vault_setup_status")
    : Promise.resolve(ready);

export const chooseVault = (): Promise<boolean> =>
  invoke<boolean>("choose_vault");

export const completeFirstRunIntro = (): Promise<boolean> =>
  invoke<boolean>("complete_first_run_intro");

export interface MigrationRecovery {
  stage: string;
  safeError: string;
  manifestFile: string | null;
  retryAvailable: boolean;
  restoreAvailable: boolean;
  createdAt: string;
}

export const getMigrationRecoveryStatus = (): Promise<{
  recovery: MigrationRecovery | null;
}> =>
  window.__TAURI_INTERNALS__
    ? invoke("migration_recovery_status")
    : Promise.resolve({ recovery: null });

export const restoreMigrationBackup = (
  manifestFile: string,
): Promise<{ recovered: boolean }> =>
  invoke("migration_recovery_restore", { manifestFile });

export const retryMigration = (): Promise<{ recovered: boolean }> =>
  invoke("migration_recovery_retry");
