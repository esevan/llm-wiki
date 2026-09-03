import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const trackedPython = execFileSync('git', ['ls-files', '*.py', '*.pyi'], {
  cwd: repositoryRoot,
  encoding: 'utf8',
}).trim();

if (trackedPython) throw new Error(`Tracked Python source remains:\n${trackedPython}`);
for (const retiredPath of ['pyproject.toml', 'uv.lock', 'llm_wiki', 'frontend/src/services/httpApplicationClient.ts']) {
  if (existsSync(join(repositoryRoot, retiredPath))) {
    throw new Error(`Retired browser artifact remains: ${retiredPath}`);
  }
}

const applicationFactory = readFileSync(
  join(repositoryRoot, 'frontend', 'src', 'services', 'applicationClient.ts'),
  'utf8',
);
if (!applicationFactory.includes('new TauriApplicationClient()') || applicationFactory.includes('HttpApplicationClient')) {
  throw new Error('Application client is not native-only');
}

console.log('verified native-only source tree');
