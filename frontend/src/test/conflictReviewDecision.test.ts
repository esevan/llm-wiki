import source from '../../public/runtime/conflicts.js?raw';
import { expect, it } from 'vitest';

const definition = source.slice(source.indexOf('function conflictReviewMarkup('), source.indexOf('async function runConflictReview('));
const markup = new Function('normalizeConflictReview', 'conflictCopy', 'conflictCardMarkup', 'esc', `
  let activeConflictReview=null;
  ${definition}
  return conflictReviewMarkup;
`)(
  (report: Record<string, unknown>) => ({ ...report, conflicts: report.conflicts ?? [] }),
  (_key: string, fallback: string) => fallback,
  () => '<article class="conflict-card"></article>',
  (value: unknown) => String(value ?? '').replace(/[&<>]/g, character => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[character]!),
);

it.each(['clear', 'insufficient_evidence', 'unknown'])('CB-033 offers No conflict for a completed zero-conflict %s report', recommended_state => {
  const root = document.createElement('div');
  root.innerHTML = markup({ conflicts: [], recommended_state }, 'solution', false);
  const clear = root.querySelector<HTMLButtonElement>('button[data-conflict-decision="clear"]');
  expect(clear?.textContent).toBe('No conflict');
  expect(clear?.getAttribute('aria-label')).toBe('No conflict');
});

it('CB-033 does not bypass per-conflict resolution when conflicts exist', () => {
  const root = document.createElement('div');
  root.innerHTML = markup({ conflicts: [{ id: 'conflict-1' }], recommended_state: 'clear' }, 'solution', false);
  expect(root.querySelector('button[data-conflict-decision="clear"]')).toBeNull();
});
