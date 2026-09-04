import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { App } from './app/App';
import { FirstRunIntro } from './features/first-run-intro/FirstRunIntro';
import { createApplicationClient } from './services/applicationClient';
import { completeFirstRunIntro } from './services/vaultSetupClient';
import { installDesktopScenario } from './test/desktopScenario';
import './theme/tokens.css';
import './theme/legacy-components.css';

const rootElement = document.getElementById('root');
if (!rootElement) throw new Error('React root was not found');

window.llmWikiApplication = createApplicationClient();
const isFirstRunIntro = new URLSearchParams(window.location.search).get('surface') === 'first-run-intro';
if (isFirstRunIntro) document.documentElement.classList.add('first-run-intro-surface');
flushSync(() => {
  createRoot(rootElement).render(
    <StrictMode>
      {isFirstRunIntro ? <FirstRunIntro onFinish={completeFirstRunIntro} /> : <App />}
    </StrictMode>,
  );
});
if (!isFirstRunIntro) installDesktopScenario();
