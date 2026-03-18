import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const root = process.cwd();

describe("frontend mascot motions cleanup", () => {
  it("removes the old motions directory once Rust owns runtime motion evaluation", () => {
    expect(existsSync(resolve(root, "src/lib/mascot/motions"))).toBe(false);
  });
});
