import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const requiredFiles = [
  path.join("third_party", "inochi2d-c", "out", "inochi2d-c.dll"),
  path.join("third_party", "inochi2d-c", "out", "inochi2d-c.lib"),
  path.join("public", "models", "Aka.inx"),
];
const requiredBundleResources = [
  {
    configPath: path.join("src-tauri", "tauri.windows.conf.json"),
    sourcePath: "../public/models/Aka.inx",
    targetPath: "models/Aka.inx",
  },
  {
    configPath: path.join("src-tauri", "tauri.windows.conf.json"),
    sourcePath: "../third_party/inochi2d-c/out/inochi2d-c.dll",
    targetPath: "inochi2d-c.dll",
  },
  {
    configPath: path.join("src-tauri", "tauri.windows.build.conf.json"),
    sourcePath: "../public/models/Aka.inx",
    targetPath: "models/Aka.inx",
  },
  {
    configPath: path.join("src-tauri", "tauri.windows.build.conf.json"),
    sourcePath: "../third_party/inochi2d-c/out/inochi2d-c.dll",
    targetPath: "inochi2d-c.dll",
  },
];

async function assertFileExists(relativePath, missing) {
  try {
    await fs.access(path.join(rootDir, relativePath));
  } catch {
    missing.push(relativePath);
  }
}

async function assertBundleResourceExists({ configPath, sourcePath, targetPath }, missing) {
  const absoluteConfigPath = path.join(rootDir, configPath);
  const config = JSON.parse(await fs.readFile(absoluteConfigPath, "utf8"));
  const resources = config.bundle?.resources;

  const mapping =
    resources && !Array.isArray(resources) ? resources[sourcePath] : undefined;

  if (mapping !== targetPath) {
    missing.push(`${configPath}: ${sourcePath} -> ${targetPath}`);
  }
}

async function main() {
  const missing = [];

  for (const file of requiredFiles) {
    await assertFileExists(file, missing);
  }

  for (const resource of requiredBundleResources) {
    await assertBundleResourceExists(resource, missing);
  }

  if (missing.length > 0) {
    throw new Error(
      [
        "Windows mascot build inputs are missing or misconfigured.",
        ...missing.map((file) => `- ${file}`),
        "Build or copy the Windows-host inochi2d-c artifacts, keep public/models/Aka.inx in git, and ensure both Windows Tauri configs bundle the mascot model and DLL resources.",
      ].join("\n"),
    );
  }
}

await main();
