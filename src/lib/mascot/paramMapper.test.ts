import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const root = process.cwd();

describe("frontend mascot motion mapper cleanup", () => {
  it("removes the old paramMapper module once Rust owns motion mapping", () => {
    expect(existsSync(resolve(root, "src/lib/mascot/paramMapper.ts"))).toBe(false);
  });

  it("removes the old mascot types module alongside the param mapper", () => {
    expect(existsSync(resolve(root, "src/lib/mascot/types.ts"))).toBe(false);
  });
});
