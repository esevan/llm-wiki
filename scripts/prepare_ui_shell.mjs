import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const outputRoot = join(repositoryRoot, 'dist');

mkdirSync(outputRoot, { recursive: true });
copyFileSync(join(repositoryRoot, 'frontend', 'index.html'), join(outputRoot, 'index.html'));
console.log('prepared native UI shell');
