import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const staticRoot = join(repositoryRoot, 'dist');
const index = readFileSync(join(staticRoot, 'index.html'), 'utf8');
const fonts = readFileSync(join(staticRoot, 'assets', 'fonts.css'), 'utf8');
const app = readFileSync(join(staticRoot, 'assets', 'app.css'), 'utf8');
const combinedCss = `${fonts}\n${app}`;

for (const family of ['Nunito', 'DM Mono', 'Noto Sans KR Variable']) {
  if (!fonts.includes(`font-family: '${family}'`)) {
    throw new Error(`Missing bundled font family: ${family}`);
  }
}
if (!index.includes('assets/fonts.css')) {
  throw new Error('index.html does not load the bundled font stylesheet');
}
if (/url\(["']?https?:/i.test(combinedCss) || /@import\s+url/i.test(combinedCss)) {
  throw new Error('A stylesheet can load a font or import from the network');
}
if (/data:font/i.test(combinedCss)) {
  throw new Error('Fonts must remain separate WOFF2 assets instead of inflating CSS');
}

const references = [...fonts.matchAll(/url\(['"]?([^)'"]+\.woff2)['"]?\)/g)].map(
  (match) => match[1],
);
if (references.length === 0) {
  throw new Error('No WOFF2 font assets were referenced');
}
for (const reference of references) {
  const file = resolve(join(staticRoot, 'assets'), reference);
  if (!existsSync(file)) {
    throw new Error(`Bundled font asset is missing: ${reference}`);
  }
}

console.log(`verified ${new Set(references).size} bundled WOFF2 font subsets`);
