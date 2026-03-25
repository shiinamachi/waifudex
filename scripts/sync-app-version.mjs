import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageJsonPath = path.join(rootDir, "package.json");
const cargoTomlPath = path.join(rootDir, "src-tauri", "Cargo.toml");
const cargoLockPath = path.join(rootDir, "src-tauri", "Cargo.lock");
const tauriConfigPath = path.join(rootDir, "src-tauri", "tauri.conf.json");

async function main() {
  const packageJson = JSON.parse(await fs.readFile(packageJsonPath, "utf8"));
  const version = packageJson.version;

  if (typeof version !== "string" || version.trim().length === 0) {
    throw new Error("package.json version must be a non-empty string");
  }

  const cargoToml = await fs.readFile(cargoTomlPath, "utf8");
  const packageVersionPattern = /^version = ".*"$/m;
  if (!packageVersionPattern.test(cargoToml)) {
    throw new Error("failed to locate package version in src-tauri/Cargo.toml");
  }

  const nextCargoToml = cargoToml.replace(packageVersionPattern, `version = "${version}"`);

  const cargoLock = await fs.readFile(cargoLockPath, "utf8");
  const lockPackagePattern =
    /(\[\[package\]\]\r?\nname = "waifudex"\r?\nversion = ")([^"]+)"/m;
  if (!lockPackagePattern.test(cargoLock)) {
    throw new Error('failed to locate package version in src-tauri/Cargo.lock');
  }

  const nextCargoLock = cargoLock.replace(lockPackagePattern, `$1${version}"`);

  const tauriConfig = JSON.parse(await fs.readFile(tauriConfigPath, "utf8"));
  tauriConfig.version = version;

  await fs.writeFile(cargoTomlPath, nextCargoToml, "utf8");
  await fs.writeFile(cargoLockPath, nextCargoLock, "utf8");
  await fs.writeFile(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 4)}\n`, "utf8");
}

await main();
