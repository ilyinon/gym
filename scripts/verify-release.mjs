import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const packageJson = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
const tauriConfig = JSON.parse(
  await readFile(join(root, "src-tauri", "tauri.conf.json"), "utf8"),
);
const cargoToml = await readFile(join(root, "src-tauri", "Cargo.toml"), "utf8");
const changelog = await readFile(join(root, "CHANGELOG.md"), "utf8");

const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargoToml)?.[1];
const versions = {
  "package.json": packageJson.version,
  "src-tauri/Cargo.toml": cargoVersion,
  "src-tauri/tauri.conf.json": tauriConfig.version,
};
const uniqueVersions = new Set(Object.values(versions));

if (uniqueVersions.size !== 1 || uniqueVersions.has(undefined)) {
  throw new Error(`Версии manifest-файлов не совпадают: ${JSON.stringify(versions)}`);
}

const version = packageJson.version;
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`Некорректная SemVer-версия: ${version}`);
}
if (
  !new RegExp(
    `^## \\[${version.replaceAll(".", "\\.")}\\] - \\d{4}-\\d{2}-\\d{2}$`,
    "m",
  ).test(changelog)
) {
  throw new Error(`В CHANGELOG.md нет датированного раздела [${version}]`);
}

if (process.env.GITHUB_REF_TYPE === "tag") {
  const tagVersion = process.env.GITHUB_REF_NAME?.replace(/^v/, "");
  if (tagVersion !== version) {
    throw new Error(
      `Тег ${process.env.GITHUB_REF_NAME} не соответствует версии ${version}`,
    );
  }
}

console.log(`Release metadata is consistent for Gym ${version}.`);
