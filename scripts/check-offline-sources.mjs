import { readFile, readdir } from "node:fs/promises";
import { dirname, extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const sourceRoots = [join(root, "src"), join(root, "src-tauri", "src")];
const sourceExtensions = new Set([".ts", ".tsx", ".rs"]);
const forbiddenSourcePatterns = [
  [/(^|[^\w])fetch\s*\(/, "fetch"],
  [/\bXMLHttpRequest\b/, "XMLHttpRequest"],
  [/\bWebSocket\b/, "WebSocket"],
];

async function sourceFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(
    entries.map((entry) => {
      const path = join(directory, entry.name);
      return entry.isDirectory() ? sourceFiles(path) : [path];
    }),
  );
  return nested.flat().filter((path) => sourceExtensions.has(extname(path)));
}

const violations = [];
for (const sourceRoot of sourceRoots) {
  for (const path of await sourceFiles(sourceRoot)) {
    const content = await readFile(path, "utf8");
    for (const [pattern, label] of forbiddenSourcePatterns) {
      if (pattern.test(content)) violations.push(`${path}: запрещён ${label}`);
    }
  }
}

const packageJson = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
for (const dependency of ["axios", "got", "ky", "node-fetch"]) {
  if (dependency in (packageJson.dependencies ?? {})) {
    violations.push(`package.json: сетевая зависимость ${dependency}`);
  }
}

const cargoToml = await readFile(join(root, "src-tauri", "Cargo.toml"), "utf8");
for (const dependency of ["reqwest", "ureq", "hyper"]) {
  if (new RegExp(`^${dependency}\\s*=`, "m").test(cargoToml)) {
    violations.push(`Cargo.toml: сетевая зависимость ${dependency}`);
  }
}

if (violations.length > 0) {
  console.error(violations.join("\n"));
  process.exitCode = 1;
} else {
  console.log("Production sources contain no network clients.");
}
