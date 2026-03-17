import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const root = process.cwd();
const appPath = resolve(root, "src/App.svelte");
const removedPaths = [
  "src/lib/components/Character.svelte",
  "src/lib/components/StatusBubble.svelte",
  "src/lib/components/StatusPanel.svelte",
  "src/lib/stores/codexStore.svelte.ts",
  "src/lib/stores/codex-state.ts",
  "src/lib/stores/codex-state.test.ts",
  "src/lib/types/codex.ts",
];

describe("App cleanup", () => {
  it("leaves App.svelte empty", () => {
    expect(readFileSync(appPath, "utf8").trim()).toBe("");
  });

  it("removes the files used only by the old App implementation", () => {
    for (const relativePath of removedPaths) {
      expect(existsSync(resolve(root, relativePath))).toBe(false);
    }
  });
});
