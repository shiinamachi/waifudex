import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(rootDir, "third_party", "inochi2d-c", "out");
const requiredFiles = ["inochi2d-c.dll", "inochi2d-c.lib"];

async function main() {
  const missing = [];

  for (const file of requiredFiles) {
    try {
      await fs.access(path.join(outDir, file));
    } catch {
      missing.push(path.join("third_party", "inochi2d-c", "out", file));
    }
  }

  if (missing.length > 0) {
    throw new Error(
      [
        "Windows build artifacts are missing.",
        ...missing.map((file) => `- ${file}`),
        "Build or copy the Windows-host inochi2d-c artifacts before running the Tauri Windows scripts.",
      ].join("\n"),
    );
  }
}

await main();
