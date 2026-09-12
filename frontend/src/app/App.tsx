import { useCallback, useEffect, useState } from "react";

import { CompassView } from "../features/compass/CompassView";
import { OverlayLayer } from "../features/overlays/OverlayLayer";
import { SearchView } from "../features/search/SearchView";
import { SettingsView } from "../features/settings/SettingsView";
import { WorkbenchView } from "../features/workbench/WorkbenchView";
import { VaultSetupView } from "../features/vault-setup/VaultSetupView";
import { MigrationRecoveryView } from "../features/vault-setup/MigrationRecoveryView";
import {
  chooseVault,
  getMigrationRecoveryStatus,
  getVaultSetupStatus,
  restoreMigrationBackup,
  retryMigration,
  type MigrationRecovery,
} from "../services/vaultSetupClient";
import { Sidebar } from "./Sidebar";

export type ViewId = "workbench" | "search" | "compass" | "ai-setup";

export function App() {
  const [activeView, setActiveView] = useState<ViewId>("workbench");
  const [vaultSetup, setVaultSetup] = useState<
    "checking" | "ready" | "required" | "choosing" | "error"
  >("checking");
  const [vaultError, setVaultError] = useState("");
  const [migrationRecovery, setMigrationRecovery] =
    useState<MigrationRecovery | null>(null);
  const [recoveryBusy, setRecoveryBusy] = useState(false);
  const vaultSetupBlocking =
    vaultSetup !== "ready" || migrationRecovery !== null;

  const checkVault = useCallback(async () => {
    setVaultSetup("checking");
    setVaultError("");
    try {
      const recovery = await getMigrationRecoveryStatus();
      if (recovery.recovery) {
        setMigrationRecovery(recovery.recovery);
        setVaultSetup("ready");
        return;
      }
      setMigrationRecovery(null);
      const status = await getVaultSetupStatus();
      setVaultSetup(status.required ? "required" : "ready");
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
      setVaultSetup("error");
    }
  }, []);

  useEffect(() => {
    void checkVault();
  }, [checkVault]);

  const selectVault = async () => {
    setVaultSetup("choosing");
    setVaultError("");
    try {
      const selected = await chooseVault();
      if (!selected) setVaultSetup("required");
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
      setVaultSetup("error");
    }
  };

  const recover = async (action: "restore" | "retry") => {
    setRecoveryBusy(true);
    setVaultError("");
    try {
      if (action === "restore") {
        const manifest = migrationRecovery?.manifestFile;
        if (!manifest) throw new Error("No verified backup is available");
        await restoreMigrationBackup(manifest);
      } else await retryMigration();
      await checkVault();
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
    } finally {
      setRecoveryBusy(false);
    }
  };

  return (
    <>
      <div className="app" inert={vaultSetupBlocking ? true : undefined}>
        <Sidebar activeView={activeView} onSelectView={setActiveView} />
        <main>
          <WorkbenchView active={activeView === "workbench"} />
          <SearchView active={activeView === "search"} />
          <CompassView active={activeView === "compass"} />
          <SettingsView active={activeView === "ai-setup"} />
        </main>
      </div>
      <div inert={vaultSetupBlocking ? true : undefined}>
        <OverlayLayer />
      </div>
      {vaultSetupBlocking &&
        (migrationRecovery ? (
          <MigrationRecoveryView
            recovery={migrationRecovery}
            busy={recoveryBusy}
            error={vaultError}
            onRestore={() => void recover("restore")}
            onRetry={() => void recover("retry")}
          />
        ) : (
          <VaultSetupView
            phase={vaultSetup === "ready" ? "checking" : vaultSetup}
            error={vaultError}
            onChoose={() => void selectVault()}
            onRetry={() => void checkVault()}
          />
        ))}
    </>
  );
}
