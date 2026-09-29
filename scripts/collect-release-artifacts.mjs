import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { basename, dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const platform = process.argv[2];
if (!new Set(["linux", "fedora", "macos"]).has(platform)) {
  throw new Error(
    "Использование: node scripts/collect-release-artifacts.mjs linux|fedora|macos [bundle-dir]",
  );
}

const defaultBundleDirectory =
  platform === "macos"
    ? join(root, "src-tauri", "target", "universal-apple-darwin", "release", "bundle")
    : join(root, "src-tauri", "target", "release", "bundle");
const bundleDirectory = resolve(process.argv[3] ?? defaultBundleDirectory);
const acceptedExtensions =
  platform === "macos"
    ? new Set([".dmg"])
    : platform === "fedora"
      ? new Set([".rpm", ".AppImage"])
      : new Set([".deb", ".AppImage"]);

async function filesBelow(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(
    entries.map((entry) => {
      const path = join(directory, entry.name);
      return entry.isDirectory() ? filesBelow(path) : [path];
    }),
  );
  return nested.flat();
}

const artifacts = (await filesBelow(bundleDirectory))
  .filter((path) => acceptedExtensions.has(extname(path)))
  .sort();
if (artifacts.length !== acceptedExtensions.size) {
  throw new Error(
    `Ожидалось ${acceptedExtensions.size} артефакта для ${platform}, найдено ${artifacts.length}`,
  );
}

const outputDirectory = join(root, "release-artifacts", platform);
await rm(outputDirectory, { force: true, recursive: true });
await mkdir(outputDirectory, { recursive: true });

const checksums = [];
for (const artifact of artifacts) {
  const name = basename(artifact);
  const destination = join(outputDirectory, name);
  await copyFile(artifact, destination);
  const digest = createHash("sha256")
    .update(await readFile(destination))
    .digest("hex");
  checksums.push(`${digest}  ${name}`);
}
await writeFile(
  join(outputDirectory, `SHA256SUMS-${platform}.txt`),
  `${checksums.join("\n")}\n`,
);

console.log(`Collected ${artifacts.length} ${platform} release artifacts.`);
