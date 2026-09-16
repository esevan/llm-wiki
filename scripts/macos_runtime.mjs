import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { access, chmod, copyFile, mkdir, realpath } from "node:fs/promises";
import path from "node:path";

const execute = promisify(execFile);
const output = async (command, args) => (await execute(command, args)).stdout;
const systemLibrary = name => name.startsWith("/System/Library/") || name.startsWith("/usr/lib/");

export async function configureOnnxEnvironment(env = process.env) {
  if (env.ORT_LIB_PATH || env.ORT_LIB_LOCATION) return;
  const compiler = await output("rustc", ["-vV"]);
  if (!compiler.includes("host: x86_64-apple-darwin")) return;
  const directory = "/usr/local/opt/onnxruntime/lib";
  try { await access(path.join(directory, "libonnxruntime.dylib")); }
  catch { throw new Error("Intel macOS needs ONNX Runtime: install onnxruntime with Homebrew or set ORT_LIB_LOCATION and ORT_PREFER_DYNAMIC_LINK=1."); }
  env.ORT_LIB_LOCATION = directory;
  env.ORT_PREFER_DYNAMIC_LINK = "1";
}

export async function libraryDependencies(binary) {
  return (await output("otool", ["-L", binary])).split("\n").slice(1)
    .map(line => line.trim().split(" (compatibility version")[0]).filter(Boolean);
}

async function resolveDependency(dependency, source, executable) {
  let candidates;
  const expand = name => name.replace(/^@loader_path(?=\/)/, path.dirname(source))
    .replace(/^@executable_path(?=\/)/, path.dirname(executable));
  if (path.isAbsolute(dependency)) candidates = [dependency];
  else if (dependency.startsWith("@loader_path/") || dependency.startsWith("@executable_path/")) candidates = [expand(dependency)];
  else if (dependency.startsWith("@rpath/")) {
    const commands = await output("otool", ["-l", source]);
    const rpaths = [...commands.matchAll(/cmd LC_RPATH\s+cmdsize \d+\s+path (.+) \(offset \d+\)/g)].map(match => expand(match[1]));
    candidates = [...rpaths, path.dirname(source)].map(base => path.join(base, dependency.slice(7)));
  } else throw new Error(`Unsupported runtime dependency ${dependency} in ${source}`);
  for (const candidate of candidates) {
    try { return await realpath(candidate); } catch { /* Try the next runpath. */ }
  }
  throw new Error(`Missing runtime dependency ${dependency} in ${source}`);
}

/** Copy the dependency closure before rewriting anything; never modify system libraries. */
export async function stageRuntime(executable, directory) {
  await mkdir(directory, { recursive: true });
  const libraries = new Map();
  const dependencies = new Map();
  const queue = [executable];
  while (queue.length) {
    const source = queue.pop();
    if (dependencies.has(source)) continue;
    const names = (await libraryDependencies(source)).filter(name => !systemLibrary(name));
    dependencies.set(source, names);
    for (const name of names) {
      const original = await resolveDependency(name, source, executable);
      const filename = path.basename(name);
      const previous = libraries.get(filename);
      if (previous && previous !== original) throw new Error(`Runtime filename collision: ${filename}`);
      libraries.set(filename, original);
      queue.push(original);
    }
  }
  const frameworks = [];
  for (const [name, source] of libraries) {
    const destination = path.join(directory, name);
    await copyFile(source, destination);
    await chmod(destination, 0o755);
    const args = dependencies.get(source).flatMap(dependency => ["-change", dependency, `@loader_path/${path.basename(dependency)}`]);
    await output("install_name_tool", [...args, "-id", `@loader_path/${name}`, destination]);
    frameworks.push(destination);
  }
  const changes = dependencies.get(executable).flatMap(dependency => ["-change", dependency, `@executable_path/../Frameworks/${path.basename(dependency)}`]);
  if (changes.length) await output("install_name_tool", [...changes, executable]);
  return frameworks;
}

export async function verifyBundledRuntime(app) {
  const executable = path.join(app, "Contents/MacOS/llm-wiki-desktop");
  const contents = await realpath(path.join(app, "Contents"));
  const pending = [executable], visited = new Set();
  while (pending.length) {
    const binary = pending.pop();
    if (visited.has(binary)) continue;
    visited.add(binary);
    for (const name of await libraryDependencies(binary)) {
      if (systemLibrary(name)) continue;
      if (!name.startsWith("@loader_path/") && !name.startsWith("@executable_path/")) {
        throw new Error(`External runtime dependency in package: ${name}`);
      }
      const resolved = await resolveDependency(name, binary, executable);
      if (!resolved.startsWith(`${contents}${path.sep}`)) throw new Error(`Runtime dependency escapes app: ${name}`);
      pending.push(resolved);
    }
  }
  return visited.size - 1;
}
