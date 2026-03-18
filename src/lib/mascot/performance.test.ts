import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const root = process.cwd();

describe("mascot performance path", () => {
  it("targets roughly 120fps in the Rust render loop", () => {
    const source = readFileSync(resolve(root, "src-tauri/src/mascot.rs"), "utf8");

    expect(source).toMatch(/Duration::from_millis\(8\)/);
    expect(source).not.toMatch(/Duration::from_millis\(16\)/);
  });

  it("removes the Svelte character component from the performance path entirely", () => {
    expect(existsSync(resolve(root, "src/lib/components/Character.svelte"))).toBe(false);
  });

  it("removes the webview frame transport module from the performance path entirely", () => {
    expect(existsSync(resolve(root, "src/lib/mascot/transport.ts"))).toBe(false);
  });
});
