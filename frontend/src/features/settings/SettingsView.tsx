import { IconButton } from '../../components/IconButton';
import { McpConnections } from './McpConnections';
import { useSyncExternalStore } from 'react';
import { useEffect, useState } from 'react';
import { chooseVault } from '../../services/vaultSetupClient';
import english from '../../../public/i18n/en.json';
import korean from '../../../public/i18n/ko.json';

const advancedTasks = [
  ['capture_assistance', 'Capture discussion and refinement'],
  ['problem_drafting', 'Capture to Problem draft'],
  ['problem_assistance', 'Problem discussion and refinement'],
  ['workbench_organization', 'Workbench organization'],
  ['solution_drafting', 'Problem to Solution draft'],
  ['solution_assistance', 'Solution discussion and refinement'],
  ['completed_solution_chat', 'Completed Solution discussion'],
  ['conflict_review', 'Conflict review'],
  ['image_summary', 'Image summary'],
  ['completion_review', 'Completion review'],
  ['completion_report', 'Completion report'],
  ['lineage_inference', 'Lineage interpretation'],
  ['problem_enrichment', 'Problem enrichment'],
] as const;

export function SettingsView({ active }: { active: boolean }) {
  const [vaultPath, setVaultPath] = useState('');
  const [vaultState, setVaultState] = useState<'loading' | 'ready' | 'error'>('loading');
  const [vaultAction, setVaultAction] = useState('');
  const [indexing, setIndexing] = useState(false);
  const ko = useSyncExternalStore(
    (changed) => { const observer = new MutationObserver(changed); observer.observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] }); return () => observer.disconnect(); },
    () => document.documentElement.lang.startsWith('ko'), () => false,
  );
  const resources = (ko ? korean : english) as Record<string, string>;
  const t = (key: string) => resources[`ai_setup.${key}`] ?? key;
  const loadVault = () => {
    setVaultState('loading');
    if (!window.llmWikiApplication) { setVaultState('error'); return; }
    void window.llmWikiApplication.request({ path: '/settings/vault' }).then(async response => {
      if (!response.ok) throw new Error('vault_settings_failed');
      setVaultPath((await response.json<{ path: string }>()).path); setVaultState('ready');
    }).catch(() => {
      setVaultPath(''); setVaultState('error');
    });
  };
  useEffect(loadVault, []);
  const regenerateEmbeddings = async () => {
    if (indexing || !window.llmWikiApplication) return;
    setIndexing(true); setVaultAction(t('vault.regenerating'));
    try {
      const response = await window.llmWikiApplication.request({ path: '/index/embeddings', method: 'POST' });
      setVaultAction(response.ok ? t('vault.regenerated') : t('vault.regenerate_failed'));
    } catch { setVaultAction(t('vault.regenerate_failed')); } finally { setIndexing(false); }
  };
  const securityNote = useSyncExternalStore(
    (changed) => {
      const observer = new MutationObserver(changed);
      observer.observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] });
      return () => observer.disconnect();
    },
    () => document.documentElement.lang.startsWith('ko')
      ? 'API 키는 이 기기의 로컬 설정 파일에만 저장됩니다. Vault나 앱 데이터베이스에는 저장되지 않습니다.'
      : 'Your API key is stored in this device’s local settings file, never in the vault or app database.',
    () => 'Your API key is stored in this device’s local settings file, never in the vault or app database.',
  );
  return (
    <section id="ai-setup" className={`view${active ? ' active' : ''}`}>
      <header className="top">
        <div><div className="eyebrow">Intelligence that organizes with you</div><h1>AI setup</h1></div>
      </header>
      <section className="result">
        <p>{securityNote}</p>
        <section className="settings-group vault-settings" aria-labelledby="vault-settings-title" aria-busy={vaultState === 'loading'}>
          <header className="vault-settings-heading">
            <h2 id="vault-settings-title">{t('vault.title')}</h2>
            {vaultState === 'ready' && <span className="vault-state">{t('vault.active')}</span>}
          </header>
          <div className="vault-location">
            <span className="vault-location-label">{t('vault.location')}</span>
            <code className={`vault-path ${vaultState === 'error' ? 'is-error' : ''}`} data-control="vault-path">{vaultPath || (vaultState === 'loading' ? t('vault.loading') : t('vault.unavailable'))}</code>
          </div>
          <p className="meta">{t('vault.description')}</p>
          <footer className="vault-actions">
            <button className="tiny" type="button" data-control="vault-change" onClick={() => void chooseVault()}>{t('vault.change')}</button>
            {vaultState === 'error' && <button className="tiny" type="button" data-control="vault-retry" onClick={loadVault}>{t('vault.retry')}</button>}
            <button className="tiny vault-secondary-action" type="button" data-control="vault-regenerate-embeddings" disabled={indexing || vaultState !== 'ready'} onClick={() => void regenerateEmbeddings()}>{indexing ? t('vault.regenerating') : t('vault.regenerate')}</button>
          </footer>
          {vaultAction && <p className="meta" role="status" aria-live="polite">{vaultAction}</p>}
        </section>
        <form className="modal" id="provider-form" data-control="provider-form">
          <fieldset className="settings-group">
            <legend data-i18n="ai_setup.connection_group">Connection</legend>
            <label htmlFor="provider-url">OpenAI-compatible endpoint</label><input id="provider-url" data-control="provider-url" required />
          </fieldset>
          <fieldset className="settings-group">
            <legend data-i18n="ai_setup.models_group">Model routing</legend>
            <label htmlFor="provider-model" title="Used for every AI task unless that task is assigned to the advanced model.">Default model <small>ⓘ</small></label>
            <input id="provider-model" data-control="provider-model" placeholder="e.g. gpt-5.6-luna" />
            <label htmlFor="provider-advanced-model" title="Used only by tasks enabled in Advanced options. If blank, those tasks use the default model.">Advanced model <small>ⓘ</small></label>
            <input id="provider-advanced-model" data-control="provider-advanced-model" placeholder="e.g. gpt-5.6-terra" />
            <details className="advanced-options" data-control="provider-advanced-options">
              <summary title="Choose which AI tasks use the advanced model instead of the default model.">Advanced options <small>ⓘ</small></summary>
              <p className="meta">Enabled tasks use the advanced model. If it is blank, they safely use the default model.</p>
              <div id="advanced-task-options">
                {advancedTasks.map(([task, label]) => <label key={task}><input type="checkbox" data-control={`provider-advanced-${task}`} data-advanced-task={task} /> {label}</label>)}
              </div>
            </details>
          </fieldset>
          <fieldset className="settings-group">
            <legend data-i18n="ai_setup.credential_group">Credential</legend>
            <label htmlFor="provider-key">API key</label><input id="provider-key" data-control="provider-key" type="password" placeholder="Leave blank to keep the stored key" />
          </fieldset>
          <footer>
            <IconButton kind="primary" data-control="provider-save" label="Save configuration" labelVisible>✓</IconButton>
            <IconButton type="button" id="provider-test" data-control="provider-test" label="Test connection and list models" labelVisible>⌁</IconButton>
          </footer>
        </form>
        <p id="provider-status" className="meta" role="status" aria-live="polite" />
        <McpConnections />
      </section>
    </section>
  );
}
