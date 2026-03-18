import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const root = process.cwd();

describe("native mascot migration scaffolding", () => {
  it("pins the upstream inochi2d-c submodule", () => {
    expect(existsSync(resolve(root, ".gitmodules"))).toBe(true);

    const source = readFileSync(resolve(root, ".gitmodules"), "utf8");
    expect(source).toMatch(/\[submodule "third_party\/inochi2d-c"\]/);
    expect(source).toMatch(/url = https:\/\/github\.com\/Inochi2D\/inochi2d-c/);
  });

  it("build script targets the yesgl dub configuration and emits into third_party out", () => {
    const source = readFileSync(resolve(root, "scripts/build-inochi2d.sh"), "utf8");

    expect(source).toMatch(/dub build --compiler=ldc2 --config=yesgl/);
    expect(source).toMatch(/third_party\/inochi2d-c\/out/);
  });

  it("bindgen wrapper includes the upstream inochi2d header", () => {
    const source = readFileSync(resolve(root, "crates/inochi2d-sys/wrapper.h"), "utf8");

    expect(source).toMatch(/#include "inochi2d.h"/);
  });
});
