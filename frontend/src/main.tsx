import '@fontsource/dm-mono/400.css';
import '@fontsource/dm-mono/500.css';
import '@fontsource/nunito/500.css';
import '@fontsource/nunito/600.css';
import '@fontsource/nunito/700.css';
import '@fontsource/nunito/800.css';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { App } from './app/App';
import { createApplicationClient } from './services/applicationClient';
import { installDesktopScenario } from './test/desktopScenario';
import './theme/tokens.css';
import './theme/legacy-components.css';

const rootElement = document.getElementById('root');
if (!rootElement) throw new Error('React root was not found');

window.llmWikiApplication = createApplicationClient();
flushSync(() => {
  createRoot(rootElement).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
});
installDesktopScenario();
