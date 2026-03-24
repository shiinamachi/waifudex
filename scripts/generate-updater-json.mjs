import fs from "node:fs/promises";
import path from "node:path";

const HELP_TEXT = `Usage:
  node ./scripts/generate-updater-json.mjs \\
    --version <semver> \\
    --installer <filename-or-url> \\
    --signature-file <path> \\
    [--release-base-url <url>] \\
    [--output <path>] \\
    [--pub-date <iso-8601>] \\
    [--notes-file <path>]

Generates a Tauri static updater manifest for windows-x86_64.

Required:
  --version           Release version to publish in latest.json
  --installer         Installer filename or full download URL
  --signature-file    Path to the generated .sig file whose contents will be embedded

URL behavior:
  --installer         If this is already a full URL, it is used as-is
  --release-base-url  If --installer is a filename, prepend this base URL

Optional:
  --output            Output path for latest.json (default: ./latest.json)
  --pub-date          ISO-8601 publication timestamp (default: current UTC time)
  --notes-file        Path to a text/markdown file to embed as release notes
  --help              Show this message
`;

function fail(message) {
  console.error(`generate-updater-json: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = {
    output: "latest.json",
    pubDate: new Date().toISOString(),
  };

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
      case "version":
        args.version = value;
        break;
      case "installer":
        args.installer = value;
        break;
      case "signature-file":
        args.signatureFile = value;
        break;
      case "release-base-url":
        args.releaseBaseUrl = value;
        break;
      case "output":
        args.output = value;
        break;
      case "pub-date":
        args.pubDate = value;
        break;
      case "notes-file":
        args.notesFile = value;
        break;
      default:
        fail(`unknown option: --${key}`);
    }

    index += 1;
  }

  return args;
}

function normalizeInstallerUrl(installer, releaseBaseUrl) {
  if (/^https?:\/\//u.test(installer)) {
    return installer;
  }

  if (!releaseBaseUrl) {
    fail("--release-base-url is required when --installer is not a full URL");
  }

  return new URL(installer, releaseBaseUrl.endsWith("/") ? releaseBaseUrl : `${releaseBaseUrl}/`)
    .toString();
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
  if (!args.installer) {
    fail("--installer is required");
  }
  if (!args.signatureFile) {
    fail("--signature-file is required");
  }

  const signature = (await fs.readFile(args.signatureFile, "utf8")).trim();
  if (signature.length === 0) {
    fail(`signature file is empty: ${args.signatureFile}`);
  }

  const release = {
    version: args.version,
    pub_date: args.pubDate,
    platforms: {
      "windows-x86_64": {
        signature,
        url: normalizeInstallerUrl(args.installer, args.releaseBaseUrl),
      },
    },
  };

  if (args.notesFile) {
    release.notes = await fs.readFile(args.notesFile, "utf8");
  }

  const outputPath = path.resolve(args.output);
  await fs.mkdir(path.dirname(outputPath), { recursive: true });
  await fs.writeFile(outputPath, `${JSON.stringify(release, null, 2)}\n`, "utf8");
}

await main();
