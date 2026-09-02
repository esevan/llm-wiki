import { IconButton } from '../../components/IconButton';

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
  return (
    <section id="ai-setup" className={`view${active ? ' active' : ''}`}>
      <header className="top">
        <div><div className="eyebrow">Intelligence that organizes with you</div><h1>AI setup</h1></div>
      </header>
      <section className="result">
        <p>Your API key is stored in macOS Keychain or Windows Credential Manager, never in the vault or app database.</p>
        <form className="modal" id="provider-form">
          <label>OpenAI-compatible endpoint</label><input id="provider-url" required />
          <label title="Used for every AI task unless that task is assigned to the advanced model.">Default model <small>ⓘ</small></label>
          <input id="provider-model" placeholder="e.g. gpt-5.6-luna" />
          <label title="Used only by tasks enabled in Advanced options. If blank, those tasks use the default model.">Advanced model <small>ⓘ</small></label>
          <input id="provider-advanced-model" placeholder="e.g. gpt-5.6-terra" />
          <details className="advanced-options">
            <summary title="Choose which AI tasks use the advanced model instead of the default model.">Advanced options <small>ⓘ</small></summary>
            <p className="meta">Enabled tasks use the advanced model. If it is blank, they safely use the default model.</p>
            <div id="advanced-task-options">
              {advancedTasks.map(([task, label]) => <label key={task}><input type="checkbox" data-advanced-task={task} /> {label}</label>)}
            </div>
          </details>
          <label>API key</label><input id="provider-key" type="password" placeholder="Leave blank to keep the stored key" />
          <footer>
            <IconButton kind="primary" label="Save configuration">✓</IconButton>
            <IconButton type="button" id="provider-test" label="Test connection and list models">⌁</IconButton>
          </footer>
        </form>
        <p id="provider-status" className="meta" />
      </section>
    </section>
  );
}
