import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const root = process.cwd();

describe("mascot transport", () => {
  it("removes the old transport module once native presentation owns the mascot", () => {
    expect(existsSync(resolve(root, "src/lib/mascot/transport.ts"))).toBe(false);
  });

  it("removes the old Character component alongside the transport module", () => {
    expect(existsSync(resolve(root, "src/lib/components/Character.svelte"))).toBe(false);
  });
});
