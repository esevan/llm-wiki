import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const frontendRoot = join(repositoryRoot, 'frontend');
const index = readFileSync(join(frontendRoot, 'index.html'), 'utf8');
const runtimeSources = [...index.matchAll(/<script defer src="(runtime\/[^"]+\.js)"><\/script>/g)]
  .map((match) => readFileSync(join(frontendRoot, 'public', match[1]), 'utf8'));

if (runtimeSources.length !== 11) {
  throw new Error(`Expected 11 feature runtime modules, found ${runtimeSources.length}`);
}

const runtime = runtimeSources.join('\n');
new Function(runtime);
if (/fetch\(['"]\/api/.test(runtime) || /HttpApplicationClient/.test(runtime)) {
  throw new Error('Native runtime contains a retired HTTP application transport');
}

console.log('native UI runtime parses without an HTTP fallback');
