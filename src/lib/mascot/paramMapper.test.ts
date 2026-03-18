import { describe, expect, it } from "vitest";

import type { RuntimeStatus } from "../contracts/generated/runtime";
import {
  createPuppetStatus,
  filterSupportedParams,
  lerpParamValue,
} from "./paramMapper";
import type { PuppetParam } from "./types";

const getParam = (params: PuppetParam[], name: string) =>
  params.find((param) => param.name === name);

describe("mascot paramMapper", () => {
  it("maps idle to a breathing pose that changes over time", () => {
    const first = createPuppetStatus("idle", 0);
    const second = createPuppetStatus("idle", Math.PI / 2);

    expect(first.runtimeStatus).toBe("idle");
    expect(first.targets).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ name: "ParamAngleX", x: 0, y: 0 }),
        expect.objectContaining({ name: "ParamBodyAngleX", x: 0, y: 0 }),
      ]),
    );

    expect(getParam(first.targets, "ParamBreath")?.y).not.toBe(
      getParam(second.targets, "ParamBreath")?.y,
    );
  });

  it("maps active statuses to distinct expressions", () => {
    const statuses: RuntimeStatus[] = ["thinking", "coding", "question", "complete"];
    const targets = statuses.map((status) => createPuppetStatus(status, 0));

    expect(getParam(targets[0].targets, "ParamAngleX")).toEqual({
      name: "ParamAngleX",
      x: -0.18,
      y: 0,
    });
    expect(getParam(targets[1].targets, "ParamMouthOpenY")).toEqual({
      name: "ParamMouthOpenY",
      x: 0,
      y: 0.32,
    });
    expect(getParam(targets[2].targets, "ParamEyeOpen")).toEqual({
      name: "ParamEyeOpen",
      x: 0,
      y: 0.98,
    });
    expect(getParam(targets[3].targets, "ParamMouthSmile")).toEqual({
      name: "ParamMouthSmile",
      x: 0,
      y: 0.85,
    });
  });

  it("treats codex_not_installed as a guarded error-like pose", () => {
    const status = createPuppetStatus("codex_not_installed", 0);

    expect(status.runtimeStatus).toBe("codex_not_installed");
    expect(getParam(status.targets, "ParamEyeOpen")).toEqual({
      name: "ParamEyeOpen",
      x: 0,
      y: 0.9,
    });
    expect(getParam(status.targets, "ParamBodyAngleX")).toEqual({
      name: "ParamBodyAngleX",
      x: -0.12,
      y: 0,
    });
  });

  it("filters out unsupported params before the renderer applies them", () => {
    const filtered = filterSupportedParams(
      [
        { name: "ParamAngleX", x: 0.2, y: 0 },
        { name: "ParamDoesNotExist", x: 1, y: 1 },
      ],
      new Set(["ParamAngleX"]),
    );

    expect(filtered).toEqual([{ name: "ParamAngleX", x: 0.2, y: 0 }]);
  });

  it("lerps and clamps each axis independently", () => {
    expect(
      lerpParamValue(
        { name: "ParamAngleX", x: -1, y: 0.25 },
        { name: "ParamAngleX", x: 1, y: 0.75 },
        0.25,
      ),
    ).toEqual({
      name: "ParamAngleX",
      x: -0.5,
      y: 0.375,
    });

    expect(
      lerpParamValue(
        { name: "ParamAngleX", x: -1, y: 0 },
        { name: "ParamAngleX", x: 1, y: 1 },
        2,
      ),
    ).toEqual({
      name: "ParamAngleX",
      x: 1,
      y: 1,
    });
  });
});
