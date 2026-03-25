import fs from "node:fs/promises";
import path from "node:path";

const HELP_TEXT = `Usage:
  node ./scripts/normalize-tauri-windows-build-config.mjs \\
    --config <path> \\
    --output <path>

Rewrites Windows Tauri build config paths to absolute paths so hosted Windows
runner working-directory differences do not break NSIS/resource resolution.
`;

function fail(message) {
  console.error(`normalize-tauri-windows-build-config: ${message}`);
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

    if (!token.startsWith("--")) {
      fail(`unexpected positional argument: ${token}`);
    }

    const key = token.slice(2);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      fail(`missing value for --${key}`);
    }

    switch (key) {
      case "config":
        args.config = value;
        break;
      case "output":
        args.output = value;
        break;
      default:
        fail(`unknown option: --${key}`);
    }

    index += 1;
  }

  return args;
}

function absolutizeMaybe(configDir, value) {
  if (typeof value !== "string" || value.length === 0) {
    return value;
  }

  if (path.isAbsolute(value)) {
    return value;
  }

  return path.resolve(configDir, value);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    process.stdout.write(HELP_TEXT);
    return;
  }

  if (!args.config) {
    fail("--config is required");
  }
  if (!args.output) {
    fail("--output is required");
  }

  const configPath = path.resolve(args.config);
  const outputPath = path.resolve(args.output);
  const configDir = path.dirname(configPath);

  const config = JSON.parse(await fs.readFile(configPath, "utf8"));

  if (config.bundle?.resources && !Array.isArray(config.bundle.resources)) {
    const nextResources = {};
    for (const [sourcePath, targetPath] of Object.entries(config.bundle.resources)) {
      nextResources[absolutizeMaybe(configDir, sourcePath)] = targetPath;
    }
    config.bundle.resources = nextResources;
  }

  const nsis = config.bundle?.windows?.nsis;
  if (nsis) {
    if ("template" in nsis) {
      nsis.template = absolutizeMaybe(configDir, nsis.template);
    }
    if ("installerIcon" in nsis) {
      nsis.installerIcon = absolutizeMaybe(configDir, nsis.installerIcon);
    }
    if ("headerImage" in nsis) {
      nsis.headerImage = absolutizeMaybe(configDir, nsis.headerImage);
    }
    if ("sidebarImage" in nsis) {
      nsis.sidebarImage = absolutizeMaybe(configDir, nsis.sidebarImage);
    }
    if ("installerHooks" in nsis) {
      nsis.installerHooks = absolutizeMaybe(configDir, nsis.installerHooks);
    }
  }

  await fs.mkdir(path.dirname(outputPath), { recursive: true });
  await fs.writeFile(outputPath, `${JSON.stringify(config, null, 2)}\n`, "utf8");
}

await main();
