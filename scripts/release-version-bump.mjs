import fs from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageJsonPath = path.join(rootDir, "package.json");
const syncScriptPath = path.join(rootDir, "scripts", "sync-app-version.mjs");

const HELP_TEXT = `Usage:
  node ./scripts/release-version-bump.mjs --version <semver>

Updates package.json to the requested version and then synchronizes
src-tauri/Cargo.toml and src-tauri/tauri.conf.json via sync-app-version.mjs.

Required:
  --version           Semantic version to publish

Optional:
  --help              Show this message
`;

function fail(message) {
  console.error(`release-version-bump: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = {};

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];

    if (token === "--help") {
      args.help = true;
      continue;
    }

    if (token === "--") {
      continue;
    }

    if (!token.startsWith("--")) {
      fail(`unexpected positional argument: ${token}`);
    }

    const key = token.slice(2);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      fail(`missing value for --${key}`);
    }

    switch (key) {
      case "version":
        args.version = value;
        break;
      default:
        fail(`unknown option: --${key}`);
    }

    index += 1;
  }

  return args;
}

function isValidSemver(version) {
  return /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/u.test(version);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    process.stdout.write(HELP_TEXT);
    return;
  }

  if (!args.version) {
    fail("--version is required");
  }

  if (!isValidSemver(args.version)) {
    fail(`invalid semantic version: ${args.version}`);
  }

  const packageJson = JSON.parse(await fs.readFile(packageJsonPath, "utf8"));
  if (packageJson.version === args.version) {
    return;
  }

  packageJson.version = args.version;
  await fs.writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`, "utf8");

  execFileSync(process.execPath, [syncScriptPath], {
    cwd: rootDir,
    stdio: "inherit",
  });
}

await main();
