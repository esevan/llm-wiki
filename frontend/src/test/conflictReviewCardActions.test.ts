import source from '../../public/runtime/solution-work.js?raw';
import { expect, it } from 'vitest';

const definition = source.split('\n').find(line => line.startsWith('featureActions=function(feature)'))!;
const render = new Function('menuButton', 'moreMenu', 'priorityButton', 'transitionMenuItem', 'manualButton', 'removeButton', 'iconButton', 'conflictCopy', `${definition}; return featureActions;`)(
  (label: string, action: string, kind = 'tiny') => `<button class="${kind}" ${action}>${label}</button>`,
  (items: string) => `<details><summary>More actions</summary>${items}</details>`,
  () => '', () => '', () => '', () => '',
  (icon: string, label: string, action: string) => `<button aria-label="${label}" ${action}>${icon}</button>`,
  (_key: string, fallback: string) => fallback,
);

it('CB-032 makes saved review viewing primary and fresh review a More action', () => {
  const html = render({ id: 'solution', state: 'proposed', conflict_state: 'unknown' });
  const root = document.createElement('div'); root.innerHTML = html;
  const primary = root.querySelector<HTMLButtonElement>(':scope > button[data-solution-action="conflict"]')!;
  expect(primary.textContent).toBe('View review result');
  expect(primary.dataset.conflictForce).toBeUndefined();
  const fresh = root.querySelector<HTMLButtonElement>('details button[data-conflict-force="true"]')!;
  expect(fresh.textContent).toBe('Run new review');
});

it('CB-032 retains Start in progress while keeping fresh review under More after a clear result', () => {
  const html = render({ id: 'solution', state: 'proposed', conflict_state: 'clear' });
  const root = document.createElement('div'); root.innerHTML = html;
  expect(root.querySelector(':scope > button[data-solution-action="stage"]')).not.toBeNull();
  expect(root.querySelector('details button[data-conflict-force="true"]')).not.toBeNull();
});
