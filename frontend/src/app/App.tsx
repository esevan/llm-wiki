import { useCallback, useEffect, useState } from 'react';

import { CompassView } from '../features/compass/CompassView';
import { OverlayLayer } from '../features/overlays/OverlayLayer';
import { SearchView } from '../features/search/SearchView';
import { SettingsView } from '../features/settings/SettingsView';
import { WorkbenchView } from '../features/workbench/WorkbenchView';
import { VaultSetupView } from '../features/vault-setup/VaultSetupView';
import { chooseVault, getVaultSetupStatus } from '../services/vaultSetupClient';
import { Sidebar } from './Sidebar';

export type ViewId = 'workbench' | 'search' | 'compass' | 'ai-setup';

export function App() {
  const [activeView, setActiveView] = useState<ViewId>('workbench');
  const [vaultSetup, setVaultSetup] = useState<'checking' | 'ready' | 'required' | 'choosing' | 'error'>('checking');
  const [vaultError, setVaultError] = useState('');
  const vaultSetupBlocking = vaultSetup !== 'ready';

  const checkVault = useCallback(async () => {
    setVaultSetup('checking');
    setVaultError('');
    try {
      const status = await getVaultSetupStatus();
      setVaultSetup(status.required ? 'required' : 'ready');
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
      setVaultSetup('error');
    }
  }, []);

  useEffect(() => {
    void checkVault();
  }, [checkVault]);

  const selectVault = async () => {
    setVaultSetup('choosing');
    setVaultError('');
    try {
      const selected = await chooseVault();
      if (!selected) setVaultSetup('required');
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
      setVaultSetup('error');
    }
  };

  return (
    <>
      <div className="app" inert={vaultSetupBlocking ? true : undefined}>
        <Sidebar activeView={activeView} onSelectView={setActiveView} />
        <main>
          <WorkbenchView active={activeView === 'workbench'} />
          <SearchView active={activeView === 'search'} />
          <CompassView active={activeView === 'compass'} />
          <SettingsView active={activeView === 'ai-setup'} />
        </main>
      </div>
      <div inert={vaultSetupBlocking ? true : undefined}>
        <OverlayLayer />
      </div>
      {vaultSetupBlocking && (
        <VaultSetupView
          phase={vaultSetup}
          error={vaultError}
          onChoose={() => void selectVault()}
          onRetry={() => void checkVault()}
        />
      )}
    </>
  );
}
