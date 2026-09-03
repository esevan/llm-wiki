import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { rename, rm, stat } from 'node:fs/promises';
import { pipeline } from 'node:stream/promises';
import { Readable } from 'node:stream';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const modelDir = path.join(root, 'src-tauri', 'resources', 'embedding-model');
const manifest = JSON.parse(await (await import('node:fs/promises')).readFile(path.join(modelDir, 'manifest.json'), 'utf8'));

const digest = async (file) => {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
};

const valid = async (file, expectedSize, expectedHash) => {
  try {
    return (await stat(file)).size === expectedSize && await digest(file) === expectedHash;
  } catch {
    return false;
  }
};

for (const asset of manifest.files) {
  const target = path.join(modelDir, asset.name);
  if (await valid(target, asset.size, asset.sha256)) {
    console.log(`embedding asset verified: ${asset.name}`);
    continue;
  }
  const partial = `${target}.part`;
  await rm(partial, { force: true });
  const url = `https://huggingface.co/${manifest.repository}/resolve/${manifest.revision}/${asset.source}`;
  console.log(`downloading embedding asset: ${asset.name}`);
  const response = await fetch(url, { headers: { 'user-agent': 'llm-wiki-build/0.1' }, redirect: 'follow' });
  if (!response.ok || !response.body) throw new Error(`Embedding download failed (${response.status}): ${asset.name}`);
  await pipeline(Readable.fromWeb(response.body), createWriteStream(partial, { flags: 'wx' }));
  if (!await valid(partial, asset.size, asset.sha256)) {
    await rm(partial, { force: true });
    throw new Error(`Embedding asset verification failed: ${asset.name}`);
  }
  await rename(partial, target);
}
