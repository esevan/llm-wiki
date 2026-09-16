import { describe, expect, it } from 'vitest';
import { assertNamedSourceControlsMapped, InteractionCoverage, mergeCoverageReports, type ControlSpec } from './interactionCoverage';
import { inventoryFamilyCount, taskInteractiveManifest } from './interactionCoverageManifest';

const controls: readonly ControlSpec[] = [{ id: 'F04.save-capture', family: 'F4', kind: 'button', selector: '#save', source: 'fixture.tsx', sourceEvidence: 'F04.save-capture', scenario: 'task-capture', effect: 'canonical capture persists' }];
const sources = new Map([['fixture.tsx', '<button data-control="F04.save-capture" id="save">Save</button>']]);

describe('interactive control coverage', () => {
  it('observes a hidden native file input only through its visible declared sibling chooser', () => {
    const file = { ...controls[0], id: 'file', kind: 'input' as const, selector: '#file', delegatedBy: 'F04.save-capture' };
    const mapped = [...controls, file];
    document.body.innerHTML = '<div><button id="save">Choose</button><input id="file" type="file" hidden></div>';
    const coverage = new InteractionCoverage(mapped, sources);
    coverage.observe(document, 'file-picker');
    expect(coverage.report().renderedIds).toEqual(['F04.save-capture', 'file']);
    document.querySelector('div')!.hidden = true;
    const hidden = new InteractionCoverage(mapped, sources);
    hidden.observe(document, 'hidden-picker');
    expect(hidden.report().renderedIds).toEqual([]);
    document.body.innerHTML = '<button id="save">Choose</button><div><input id="file" type="file" hidden></div>';
    const unrelated = new InteractionCoverage(mapped, sources);
    unrelated.observe(document, 'unrelated-picker');
    expect(unrelated.report().notRendered).toContain('file');
  });

  it('keeps individual Task controls distinct from the 68-family inventory', () => {
    expect(inventoryFamilyCount).toBe(68);
    expect(taskInteractiveManifest.length).toBeGreaterThan(68);
    expect(new Set(taskInteractiveManifest.map(control => control.id)).size).toBe(taskInteractiveManifest.length);
    expect(taskInteractiveManifest.every(control => control.selector.length > 0 && control.effect.length > 0)).toBe(true);
    expect(taskInteractiveManifest.find(control => control.id === 'chat-preview-detail')?.disabledReason).toMatch(/Detail tab/);
    expect(taskInteractiveManifest.find(control => control.id === 'chat-ask')?.disabledReason).toMatch(/streaming/);
  });
  it('counts rendered, exercised, and independently asserted controls without a fixed percentage', () => {
    document.body.innerHTML = '<button id="save">Save</button>'; let persisted = false;
    const coverage = new InteractionCoverage(controls, sources); coverage.observe(document, 'task-capture');
    coverage.interact('F04.save-capture', () => { persisted = true; });
    coverage.assertEffect('F04.save-capture', () => persisted);
    expect(coverage.assertComplete()).toMatchObject({ inventoryCount: 1, scannedSourceControls: 1, rendered: 1, exercised: 1, asserted: 1 });
  });
  it('rejects unknown enabled rendered controls', () => {
    document.body.innerHTML = '<button id="save"></button><button id="new"></button>'; const coverage = new InteractionCoverage(controls, sources); coverage.observe(document, 'task-capture'); coverage.interact('F04.save-capture', () => {}); coverage.assertEffect('F04.save-capture', () => true);
    expect(() => coverage.assertComplete()).toThrow('unknown enabled');
  });
  it('rejects controls that are not exercised or whose click has no effect', () => {
    document.body.innerHTML = '<button id="save"></button>'; const coverage = new InteractionCoverage(controls, sources); coverage.observe(document, 'task-capture');
    expect(() => coverage.assertComplete()).toThrow('unexercised'); coverage.interact('F04.save-capture', () => {});
    expect(() => coverage.assertComplete()).toThrow('no semantic effect');
    expect(() => coverage.assertEffect('F04.save-capture', () => false)).toThrow('no independent persisted or semantic effect');
  });
  it('requires source evidence and a disabled-state precondition', () => {
    document.body.innerHTML = '<button id="save" disabled></button>'; const coverage = new InteractionCoverage([{ ...controls[0], sourceEvidence: 'missing' }], sources); coverage.observe(document, 'task-capture', 'busy');
    expect(coverage.report()).toMatchObject({ missingSourceEvidence: ['F04.save-capture'], undocumentedDisabled: ['F04.save-capture (busy)'] });
  });
  it('rejects a conditional named source control before any fixture renders it', () => {
    expect(() => assertNamedSourceControlsMapped('<button data-control="F04.save-capture" /><button id="conditional-new" />', controls)).toThrow('conditional-new');
    expect(() => assertNamedSourceControlsMapped('<button id="reviewed-offline" />', controls, ['reviewed-offline'])).not.toThrow();
    expect(() => assertNamedSourceControlsMapped('<button data-control="F04.save-capture" /><button onClick={() => save()}>New conditional</button>', controls)).toThrow('unnamed-button-2');
  });
  it('does not let spread props or dynamic identities bypass the source inventory', () => {
    expect(() => assertNamedSourceControlsMapped('<button {...props}>New conditional</button>', controls)).toThrow('unnamed-button-1');
    expect(() => assertNamedSourceControlsMapped('<button data-control={id}>New conditional</button>', controls)).toThrow('id');
    expect(() => assertNamedSourceControlsMapped('<IconButton {...props}>New conditional</IconButton>', controls)).toThrow('unnamed-IconButton-1');
  });
  it('allows only reviewed delegating definitions and requires identities at every wrapper callsite', () => {
    const definition = 'export function IconButton(props) { return <button data-tooltip={label} aria-label={label} {...props}>icon</button> }';
    expect(() => assertNamedSourceControlsMapped(`${definition}<IconButton data-control="F04.save-capture" />`, controls)).not.toThrow();
    expect(() => assertNamedSourceControlsMapped(`${definition}<IconButton label="Save" />`, controls)).toThrow('unnamed-IconButton');
    const actionDefinition = 'function IconAction(props) { return <button id={id} data-control={id}>icon</button> }';
    expect(() => assertNamedSourceControlsMapped(`${actionDefinition}<IconAction id="F04.save-capture" />`, controls)).not.toThrow();
    expect(() => assertNamedSourceControlsMapped(`${actionDefinition}<IconAction label="Close" />`, controls)).toThrow('unnamed-IconAction');
  });
  it.each([
    ['anchor', '<a href="/new">New</a>'],
    ['button role', '<div role="button">New</div>'],
    ['checkbox role', '<span role="checkbox">New</span>'],
    ['switch role', '<x-toggle role="switch" />'],
    ['link role', '<div role="link">New</div>'],
    ['tab role', '<li role="tab">New</li>'],
    ['menuitem role', '<div role="menuitem">New</div>'],
    ['content editable custom element', '<wiki-editor contentEditable />'],
  ])('rejects an unmapped %s source control', (_label, source) => {
    expect(() => assertNamedSourceControlsMapped(source, controls)).toThrow('Unmapped named source controls');
  });
  it('unions process reports by control id instead of adding duplicate counts', () => {
    document.body.innerHTML = '<button id="save"></button>';
    const first = new InteractionCoverage(controls, sources); first.observe(document, 'first');
    const second = new InteractionCoverage(controls, sources); second.observe(document, 'second');
    second.interact('F04.save-capture', () => {}); second.assertEffect('F04.save-capture', () => true);
    expect(mergeCoverageReports([first.report(), second.report()])).toMatchObject({
      rendered: 1, exercised: 1, asserted: 1, unexercised: [], effectMissing: [], notRendered: [],
    });
  });
});
