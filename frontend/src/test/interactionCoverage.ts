/** A scenario-owned audit trail for every individual interactive DOM control. */
export type ControlKind = 'button' | 'input' | 'select' | 'textarea' | 'form' | 'details' | 'tab' | 'dialog-close' | 'menu' | 'keyboard';

export type ControlSpec = {
  id: string;
  family: `F${number}`;
  kind: ControlKind;
  selector: string;
  source: string;
  sourceEvidence: string;
  scenario: string;
  effect: string;
  disabledReason?: string;
};

export type CoverageReport = {
  inventoryCount: number;
  scannedSourceControls: number;
  rendered: number;
  exercised: number;
  asserted: number;
  unknownEnabled: string[];
  missingSourceEvidence: string[];
  unexercised: string[];
  effectMissing: string[];
  notRendered: string[];
  undocumentedDisabled: string[];
  renderedIds: string[];
  exercisedIds: string[];
  assertedIds: string[];
};

const INTERACTIVE = 'button,input:not([type="hidden"]),select,textarea,details,a[href],[role="button"],[role="checkbox"],[role="switch"],[role="link"],[role="tab"],[role="menuitem"],[contenteditable="true"],[data-interaction-keyboard]';

const nativeInteractiveTags = new Set(['button', 'input', 'select', 'textarea', 'details']);
const reviewedDelegatingControls = [
  {
    component: 'IconButton',
    definition: 'export function IconButton(',
    openingEvidence: 'data-tooltip={label} aria-label={label} {...props}',
  },
  {
    component: 'IconAction',
    definition: 'function IconAction(',
    openingEvidence: 'id={id} data-control={id}',
  },
] as const;

function hasStaticInteractiveRole(attrs: string) {
  return /\brole\s*=\s*(?:["'](?:button|checkbox|switch|link|tab|menuitem)["']|\{["'](?:button|checkbox|switch|link|tab|menuitem)["']\})/.test(attrs);
}

function isContentEditable(attrs: string) {
  return /\bcontentEditable(?:\s*=\s*(?:["']?true["']?|\{true\}))?(?=\s|\/?>)/i.test(attrs);
}

/** Inventories shipping control syntax, including interactive ARIA/custom elements. */
function sourceControlNodes(source: string) {
  const starts = [...source.matchAll(/<([A-Za-z][A-Za-z0-9.-]*)\b/g)];
  let interactiveIndex = 0;
  return starts.flatMap(match => {
    let quote = '', braces = 0, end = match.index! + match[0].length;
    for (; end < source.length; end += 1) {
      const char = source[end], previous = source[end - 1];
      if (quote) { if (char === quote && previous !== '\\') quote = ''; continue; }
      if (char === '"' || char === "'" || char === '`') { quote = char; continue; }
      if (char === '{') { braces += 1; continue; }
      if (char === '}') { braces = Math.max(0, braces - 1); continue; }
      if (char === '>' && braces === 0) break;
    }
    const opening = source.slice(match.index, end + 1);
    const attrs = opening.slice(match[0].length, -1);
    const tag = match[1];
    const wrapper = reviewedDelegatingControls.find(item => item.component === tag);
    const interactive = nativeInteractiveTags.has(tag)
      || Boolean(wrapper)
      || (tag === 'a' && /\bhref\s*=/.test(attrs))
      || hasStaticInteractiveRole(attrs)
      || isContentEditable(attrs);
    if (!interactive) return [];
    interactiveIndex += 1;
    if (tag === 'input' && /\btype\s*=\s*["']hidden["']/.test(attrs)) return [];
    const delegatedDefinition = reviewedDelegatingControls.some(item =>
      tag === 'button'
      && source.includes(item.definition)
      && opening.includes(item.openingEvidence));
    if (delegatedDefinition) return [];
    const identity = attrs.match(/(?:^|\s)(?:data-control|id)\s*=\s*(?:["']([^"']+)["']|\{["']([^"']+)["']\}|\{([^}]+)\})/)?.slice(1).find(Boolean);
    return [{ opening: opening.replace(/\s+/g, ' '), identity: identity ?? `unnamed-${tag}-${interactiveIndex}` }];
  });
}

export function scanInteractiveSource(source: string) {
  return sourceControlNodes(source).length + [...source.matchAll(/(?:onKeyDown|addEventListener\(["']keydown)/g)].length;
}

/** Static guard for named controls, including conditional markup never reached by a fixture. */
export function assertNamedSourceControlsMapped(source: string, controls: readonly ControlSpec[], reviewedUnreachable: readonly string[] = []) {
  const nodes = sourceControlNodes(source);
  const unknown = nodes
    .filter(node => {
      const directlyMapped = controls.some(control => node.opening.includes(control.sourceEvidence) || node.opening.includes(control.id));
      const dynamicPrefix = node.identity?.includes('${') ? node.identity.replace(/^[`"']/, '').split('${', 1)[0] : '';
      const dynamicallyMapped = dynamicPrefix.length > 0 && controls.some(control => control.id.startsWith(dynamicPrefix));
      return !directlyMapped && !dynamicallyMapped && !reviewedUnreachable.some(evidence => node.opening.includes(evidence));
    })
    .map(node => node.identity);
  if (unknown.length) throw new Error(`Unmapped named source controls: ${unknown.join(', ')}`);
}

export class InteractionCoverage {
  private rendered = new Set<string>();
  private exercised = new Set<string>();
  private asserted = new Set<string>();
  private unknown = new Set<string>();
  private undocumentedDisabled = new Set<string>();
  private readonly missingSourceEvidence: string[];
  private readonly scannedSourceControls: number;

  constructor(private readonly controls: readonly ControlSpec[], sources: ReadonlyMap<string, string>) {
    const duplicateIds = controls.map(control => control.id).filter((id, index, ids) => ids.indexOf(id) !== index);
    if (duplicateIds.length) throw new Error(`Interactive control ids must be unique: ${[...new Set(duplicateIds)].join(', ')}`);
    this.missingSourceEvidence = controls.filter(control => !sources.get(control.source)?.includes(control.sourceEvidence)).map(control => control.id);
    this.scannedSourceControls = [...new Set(controls.map(control => control.source))].reduce((count, path) => count + scanInteractiveSource(sources.get(path) ?? ''), 0);
  }

  observe(root: ParentNode, scenario: string, state = 'rendered') {
    for (const element of root.querySelectorAll(INTERACTIVE)) {
      if (element.closest('dialog:not([open]),[hidden],.view:not(.active)')) continue;
      const matches = this.controls.filter(control => element.matches(control.selector));
      const disabled = (element as HTMLButtonElement).disabled || element.getAttribute('aria-disabled') === 'true';
      if (!matches.length && !disabled) this.unknown.add(`${scenario}:${element.tagName.toLowerCase()}#${element.id || 'anonymous'}`);
      for (const control of matches) {
        this.rendered.add(control.id);
        if (disabled && !control.disabledReason) this.undocumentedDisabled.add(`${control.id} (${state})`);
      }
    }
  }

  interact(id: string, action: () => void) {
    this.require(id);
    if (!this.rendered.has(id)) throw new Error(`${id} was interacted with before it was observed in a rendered state`);
    action(); this.exercised.add(id);
  }

  assertEffect(id: string, effect: () => boolean) {
    this.require(id);
    if (!this.exercised.has(id)) throw new Error(`${id} needs an interaction before its effect assertion`);
    if (effect() !== true) throw new Error(`${id} click produced no independent persisted or semantic effect`);
    this.asserted.add(id);
  }

  report(): CoverageReport {
    const active = this.controls.filter(control => this.rendered.has(control.id));
    return {
      inventoryCount: this.controls.length, scannedSourceControls: this.scannedSourceControls,
      rendered: this.rendered.size, exercised: this.exercised.size, asserted: this.asserted.size,
      unknownEnabled: [...this.unknown], missingSourceEvidence: this.missingSourceEvidence,
      unexercised: active.filter(control => !this.exercised.has(control.id)).map(control => control.id),
      effectMissing: active.filter(control => this.exercised.has(control.id) && !this.asserted.has(control.id)).map(control => control.id),
      notRendered: this.controls.filter(control => !this.rendered.has(control.id)).map(control => control.id),
      undocumentedDisabled: [...this.undocumentedDisabled],
      renderedIds: [...this.rendered], exercisedIds: [...this.exercised], assertedIds: [...this.asserted],
    };
  }

  assertComplete() {
    const report = this.report();
    const failures = [report.unknownEnabled.length && `unknown enabled: ${report.unknownEnabled.join(', ')}`,
      report.missingSourceEvidence.length && `source drift: ${report.missingSourceEvidence.join(', ')}`,
      report.unexercised.length && `unexercised: ${report.unexercised.join(', ')}`,
      report.effectMissing.length && `no semantic effect: ${report.effectMissing.join(', ')}`,
      report.notRendered.length && `declared controls not rendered: ${report.notRendered.join(', ')}`,
      report.undocumentedDisabled.length && `disabled without precondition: ${report.undocumentedDisabled.join(', ')}`].filter(Boolean);
    if (failures.length) throw new Error(`Interactive coverage incomplete; ${failures.join('; ')}`);
    return report;
  }

  private require(id: string) { if (!this.controls.some(control => control.id === id)) throw new Error(`Unknown control id: ${id}`); }
}

/** Combines scenario-process reports by control id; counts are never summed. */
export function mergeCoverageReports(reports: readonly CoverageReport[]): CoverageReport {
  const union = (key: 'renderedIds' | 'exercisedIds' | 'assertedIds' | 'unknownEnabled' | 'missingSourceEvidence' | 'undocumentedDisabled') =>
    [...new Set(reports.flatMap(report => report[key]))];
  const renderedIds = union('renderedIds'), exercisedIds = union('exercisedIds'), assertedIds = union('assertedIds');
  const inventoryCount = Math.max(0, ...reports.map(report => report.inventoryCount));
  const missing = (key: 'unexercised' | 'effectMissing' | 'notRendered') => [...new Set(reports.flatMap(report => report[key]))];
  return {
    inventoryCount,
    scannedSourceControls: Math.max(0, ...reports.map(report => report.scannedSourceControls)),
    rendered: renderedIds.length,
    exercised: exercisedIds.length,
    asserted: assertedIds.length,
    renderedIds,
    exercisedIds,
    assertedIds,
    unknownEnabled: union('unknownEnabled'),
    missingSourceEvidence: union('missingSourceEvidence'),
    undocumentedDisabled: union('undocumentedDisabled'),
    unexercised: missing('unexercised').filter(id => renderedIds.includes(id) && !exercisedIds.includes(id)),
    effectMissing: missing('effectMissing').filter(id => exercisedIds.includes(id) && !assertedIds.includes(id)),
    notRendered: missing('notRendered').filter(id => !renderedIds.includes(id)),
  };
}
