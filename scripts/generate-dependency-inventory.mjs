import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

import licenseTexts from "../src/licenses/license-texts.js";

const execFileAsync = promisify(execFile);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(__dirname, "..");
const outputPath = path.join(projectRoot, "src", "generated", "dependencies.json");

const rustSections = [
  {
    key: "tauriClient",
    label: "Tauri client",
    manifestPath: "src-tauri/Cargo.toml",
  },
  {
    key: "inochi2dSys",
    label: "inochi2d-sys",
    manifestPath: "crates/inochi2d-sys/Cargo.toml",
  },
  {
    key: "waifudexInox2dWasm",
    label: "waifudex-inox2d-wasm",
    manifestPath: "crates/waifudex-inox2d-wasm/Cargo.toml",
  },
  {
    key: "waifudexMascot",
    label: "waifudex-mascot",
    manifestPath: "crates/waifudex-mascot/Cargo.toml",
  },
];

function isSpdxExpression(value) {
  if (typeof value !== "string") {
    return false;
  }

  const normalized = value.trim();
  if (normalized.length === 0) {
    return false;
  }

  if (normalized.startsWith("SEE LICENSE IN ")) {
    return false;
  }

  if (normalized === "UNLICENSED") {
    return false;
  }

  return /^[A-Za-z0-9-.+():/ ]+$/.test(normalized);
}

function splitLicenseExpression(value) {
  const normalized = value.trim();
  if (normalized.length === 0) {
    return [];
  }

  if (normalized.includes(" AND ")) {
    return [normalized];
  }

  const parts = normalized
    .split(/\s+OR\s+|\s*\/\s*/u)
    .map((part) => part.trim())
    .filter((part) => part.length > 0);

  if (parts.length === 0) {
    return [normalized];
  }

  return Array.from(new Set(parts));
}

function normalizeLicense(value) {
  if (typeof value === "string" && isSpdxExpression(value)) {
    return splitLicenseExpression(value);
  }

  if (value && typeof value === "object" && typeof value.type === "string") {
    const type = value.type.trim();
    if (isSpdxExpression(type)) {
      return splitLicenseExpression(type);
    }
  }

  if (Array.isArray(value)) {
    const normalized = value
      .map((item) => normalizeLicense(item))
      .flat()
      .filter((item) => item !== undefined);
    if (normalized.length > 0) {
      return Array.from(new Set(normalized));
    }
  }

  return undefined;
}

function normalizeRepository(value) {
  let url;

  if (typeof value === "string") {
    url = value;
  } else if (value && typeof value === "object" && typeof value.url === "string") {
    url = value.url;
  }

  if (!url) {
    return undefined;
  }

  const normalized = url
    .replace(/^github:/, "https://github.com/")
    .replace(/^git\+/, "")
    .replace(/^git:\/\/github\.com\//, "https://github.com/")
    .replace(/\.git$/, "");

  if (/^[^/:]+\/[^/]+$/.test(normalized)) {
    return `https://github.com/${normalized}`;
  }

  return normalized;
}

function normalizeSummaryToken(token) {
  const normalized = token.replace(/^[()]+|[()]+$/g, "").trim();
  return normalized.length > 0 ? normalized : null;
}

function expandSummaryLicenses(licenses) {
  if (!licenses || licenses.length === 0) {
    return ["Unknown"];
  }

  const expanded = licenses
    .flatMap((license) => license.split(/\s+OR\s+|\s+AND\s+|\s*\/\s*/u))
    .map(normalizeSummaryToken)
    .filter((license) => license !== null);

  return expanded.length > 0 ? Array.from(new Set(expanded)) : ["Unknown"];
}

function normalizeAuthor(value) {
  if (typeof value === "string") {
    return value.trim() || undefined;
  }

  if (value && typeof value === "object" && typeof value.name === "string") {
    return value.name.trim() || undefined;
  }

  if (Array.isArray(value)) {
    const authors = value
      .map((item) => normalizeAuthor(item))
      .filter((item) => item !== undefined);
    if (authors.length > 0) {
      return authors.join(", ");
    }
  }

  return undefined;
}

function sortEntries(entries) {
  entries.sort((left, right) => {
    const byName = left.name.localeCompare(right.name);
    if (byName !== 0) {
      return byName;
    }
    return left.version.localeCompare(right.version);
  });
  return entries;
}

function toEntry(pkg, extra = {}) {
  const entry = {
    name: pkg.name,
    version: pkg.version,
    ...extra,
  };

  const license = normalizeLicense(pkg.license ?? pkg.licenses);
  if (license) {
    entry.license = license;
  }

  const repository = normalizeRepository(pkg.repository);
  if (repository) {
    entry.repository = repository;
  }

  const author = normalizeAuthor(pkg.author ?? pkg.authors);
  if (author) {
    entry.author = author;
  }

  return entry;
}

async function readJson(filePath) {
  const raw = await fs.readFile(filePath, "utf8");
  return JSON.parse(raw);
}

async function resolvePackageManifest(packageName, fromFile) {
  const packagePathParts = packageName.split("/");
  let currentDirectory = await fs.realpath(path.dirname(fromFile));

  while (true) {
    const candidatePath =
      path.basename(currentDirectory) === "node_modules"
        ? path.join(currentDirectory, ...packagePathParts, "package.json")
        : path.join(currentDirectory, "node_modules", ...packagePathParts, "package.json");

    try {
      await fs.access(candidatePath);
      return fs.realpath(candidatePath);
    } catch {
      const parentDirectory = path.dirname(currentDirectory);
      if (parentDirectory === currentDirectory) {
        throw new Error(`Unable to resolve ${packageName} from ${fromFile}`);
      }
      currentDirectory = parentDirectory;
    }
  }
}

async function collectFrontendSection() {
  const packageJsonPath = path.join(projectRoot, "package.json");
  const packageJson = await readJson(packageJsonPath);
  const dependencyEntries = Object.entries(packageJson.dependencies ?? {}).sort(([left], [right]) =>
    left.localeCompare(right),
  );
  const roots = [];
  const packages = [];
  const seenPackageKeys = new Set();
  const seenManifestPaths = new Set();
  const queue = [];

  for (const [name, specifier] of dependencyEntries) {
    const manifestPath = await resolvePackageManifest(name, packageJsonPath);
    const manifest = await readJson(manifestPath);
    roots.push(toEntry(manifest, { specifier }));
    queue.push({ manifestPath, manifest });
  }

  while (queue.length > 0) {
    const current = queue.shift();
    if (!current || seenManifestPaths.has(current.manifestPath)) {
      continue;
    }

    seenManifestPaths.add(current.manifestPath);

    const packageKey = `${current.manifest.name}@${current.manifest.version}`;
    if (!seenPackageKeys.has(packageKey)) {
      seenPackageKeys.add(packageKey);
      packages.push(toEntry(current.manifest));
    }

    const dependencyNames = [
      ...Object.keys(current.manifest.dependencies ?? {}),
      ...Object.keys(current.manifest.optionalDependencies ?? {}),
    ].sort();

    for (const dependencyName of dependencyNames) {
      let dependencyManifestPath;
      try {
        dependencyManifestPath = await resolvePackageManifest(
          dependencyName,
          current.manifestPath,
        );
      } catch (error) {
        if (dependencyName in (current.manifest.optionalDependencies ?? {})) {
          continue;
        }
        throw error;
      }

      const dependencyManifest = await readJson(dependencyManifestPath);
      queue.push({
        manifestPath: dependencyManifestPath,
        manifest: dependencyManifest,
      });
    }
  }

  return {
    label: "Frontend",
    manifestPath: "package.json",
    roots: sortEntries(roots),
    packages: sortEntries(packages),
  };
}

async function runCargoMetadata(manifestPath) {
  const absoluteManifestPath = path.join(projectRoot, manifestPath);
  const { stdout } = await execFileAsync(
    "cargo",
    ["metadata", "--manifest-path", absoluteManifestPath, "--format-version", "1", "--locked"],
    { cwd: projectRoot, maxBuffer: 64 * 1024 * 1024 },
  );

  const jsonStart = stdout.indexOf("{");
  if (jsonStart === -1) {
    throw new Error(`cargo metadata did not return JSON for ${manifestPath}`);
  }

  return JSON.parse(stdout.slice(jsonStart));
}

function isIncludedRustDependency(depKinds) {
  return depKinds.some((item) => item.kind === null || item.kind === "build");
}

function buildRustSection(section, metadata) {
  const packageById = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
  const nodeById = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const workspacePackageIds = new Set(
    metadata.packages
      .filter((pkg) => pkg.source === null)
      .map((pkg) => pkg.id),
  );
  const rootId = metadata.resolve.root;
  const rootNode = nodeById.get(rootId);
  const roots = [];
  const packages = [];
  const seenPackageIds = new Set();
  const queue = [];

  for (const dependency of rootNode?.deps ?? []) {
    if (!isIncludedRustDependency(dependency.dep_kinds)) {
      continue;
    }

    const pkg = packageById.get(dependency.pkg);
    if (!pkg || workspacePackageIds.has(pkg.id)) {
      continue;
    }

    roots.push(toEntry(pkg));
    queue.push(pkg.id);
  }

  while (queue.length > 0) {
    const packageId = queue.shift();
    if (!packageId || seenPackageIds.has(packageId)) {
      continue;
    }

    seenPackageIds.add(packageId);
    const pkg = packageById.get(packageId);
    if (!pkg) {
      continue;
    }

    packages.push(toEntry(pkg));

    const node = nodeById.get(packageId);
    for (const dependency of node?.deps ?? []) {
      if (!isIncludedRustDependency(dependency.dep_kinds)) {
        continue;
      }

      const dependencyPackage = packageById.get(dependency.pkg);
      if (!dependencyPackage || workspacePackageIds.has(dependencyPackage.id)) {
        continue;
      }

      queue.push(dependencyPackage.id);
    }
  }

  return {
    label: section.label,
    manifestPath: section.manifestPath,
    roots: sortEntries(roots),
    packages: sortEntries(packages),
  };
}

async function collectRustSections() {
  const result = {};

  for (const section of rustSections) {
    const metadata = await runCargoMetadata(section.manifestPath);
    result[section.key] = buildRustSection(section, metadata);
  }

  return result;
}

async function writeInventory() {
  const frontend = await collectFrontendSection();
  const rust = await collectRustSections();
  const inventory = {
    frontend,
    ...rust,
  };
  const summaryLicenses = new Set(
    Object.values(inventory).flatMap((section) =>
      section.packages.flatMap((pkg) => expandSummaryLicenses(pkg.license)),
    ),
  );
  const missingLicenseTexts = Array.from(summaryLicenses)
    .filter((license) => typeof licenseTexts[license] !== "string" || licenseTexts[license].trim().length === 0)
    .sort();

  if (missingLicenseTexts.length > 0) {
    throw new Error(
      `Missing license text entries for: ${missingLicenseTexts.join(", ")}`,
    );
  }

  await fs.mkdir(path.dirname(outputPath), { recursive: true });
  await fs.writeFile(outputPath, `${JSON.stringify(inventory, null, 2)}\n`, "utf8");
}

writeInventory().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
