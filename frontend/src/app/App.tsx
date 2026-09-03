import { useState } from 'react';

import { CompassView } from '../features/compass/CompassView';
import { OverlayLayer } from '../features/overlays/OverlayLayer';
import { SearchView } from '../features/search/SearchView';
import { SettingsView } from '../features/settings/SettingsView';
import { WorkbenchView } from '../features/workbench/WorkbenchView';
import { Sidebar } from './Sidebar';

export type ViewId = 'workbench' | 'search' | 'compass' | 'ai-setup';

export function App() {
  const [activeView, setActiveView] = useState<ViewId>('workbench');

  return (
    <>
      <div className="app">
        <Sidebar activeView={activeView} onSelectView={setActiveView} />
        <main>
          <WorkbenchView active={activeView === 'workbench'} />
          <SearchView active={activeView === 'search'} />
          <CompassView active={activeView === 'compass'} />
          <SettingsView active={activeView === 'ai-setup'} />
        </main>
      </div>
      <OverlayLayer />
    </>
  );
}
