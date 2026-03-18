import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const root = process.cwd();

describe("mascot transport", () => {
  it("uses Aka.inx as the default mascot asset path", () => {
    const source = readFileSync(resolve(root, "src/lib/mascot/transport.ts"), "utf8");

    expect(source).toMatch(/modelPath = "\/models\/Aka\.inx"/);
  });

  it("subscribes to the mascot frame event and normalizes RGBA payloads", () => {
    const source = readFileSync(resolve(root, "src/lib/mascot/transport.ts"), "utf8");

    expect(source).toMatch(/waifudex:\/\/mascot-frame/);
    expect(source).toMatch(/new Uint8ClampedArray/);
    expect(source).toMatch(/invoke<.*>\("init_mascot"/s);
    expect(source).toMatch(/invoke<.*>\("update_mascot_params"/s);
    expect(source).toMatch(/invoke<.*>\("resize_mascot"/s);
    expect(source).toMatch(/invoke<.*>\("dispose_mascot"/s);
  });
});
