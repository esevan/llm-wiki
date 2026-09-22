import { describe, expect, it } from 'vitest';
import index from '../../index.html?raw';

import { assertNamedSourceControlsMapped, scanInteractiveSource } from './interactionCoverage';
import { taskInteractiveManifest } from './interactionCoverageManifest';
import { legacyReviewedUnreachableSourceEvidence } from './legacyReachability';
import { createInteractionCoverage, productionInteractiveSources } from './productionInteractiveSources';

describe('shipping interactive source inventory', () => {
  it('discovers every shipping component and index-loaded runtime instead of trusting a fixed import list', () => {
    const componentModules = import.meta.glob('../**/*.{ts,tsx}', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
    const runtimeModules = import.meta.glob('../../public/runtime/*.js', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
    const discoveredComponents = Object.entries(componentModules)
      .filter(([modulePath, source]) => !modulePath.startsWith('./') && !modulePath.includes('/test/') && !/\.(?:test|spec)\.tsx?$/.test(modulePath) && scanInteractiveSource(source) > 0)
      .map(([modulePath]) => `frontend/src/${modulePath.replace(/^\.\.\//, '')}`);
    const loadedRuntime = [...index.matchAll(/<script\s+defer\s+src="(runtime\/[^"?]+\.js)"/g)]
      .map(match => `frontend/public/${match[1]}`);
    expect(Object.keys(runtimeModules).map(modulePath => `frontend/public/runtime/${modulePath.split('/').at(-1)}`).sort()).toEqual(loadedRuntime.sort());
    const registered = [...productionInteractiveSources.keys()];
    expect(registered.filter(item => item.startsWith('frontend/src/')).sort()).toEqual(discoveredComponents.sort());
    expect(registered.filter(item => item.startsWith('frontend/public/runtime/')).sort()).toEqual(loadedRuntime.sort());
  });

  it('rejects unnamed or unmapped controls, including conditional source markup', () => {
    expect(productionInteractiveSources.size).toBe(31);
    const failures: string[] = [];
    for (const [path, source] of productionInteractiveSources) {
      const controls = taskInteractiveManifest.filter(control => control.source === path);
      const reviewed = legacyReviewedUnreachableSourceEvidence[path] ?? [];
      try { assertNamedSourceControlsMapped(source, controls, reviewed); }
      catch (error) { failures.push(`${path}: ${String(error)}`); }
    }
    expect(failures).toEqual([]);
    expect(createInteractionCoverage().report().missingSourceEvidence).toEqual([]);
  });
});
