import type { RuntimeStatus } from "../contracts/generated/runtime";

import type { PuppetParam, PuppetStatus } from "./types";

const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

const param = (name: string, x: number, y: number): PuppetParam => ({ name, x, y });

const STATUS_TARGETS: Record<RuntimeStatus, PuppetParam[]> = {
  idle: [
    param("ParamAngleX", 0, 0),
    param("ParamBodyAngleX", 0, 0),
    param("ParamEyeOpen", 0, 1),
    param("ParamMouthOpenY", 0, 0.08),
  ],
  codex_not_installed: [
    param("ParamBodyAngleX", -0.12, 0),
    param("ParamEyeOpen", 0, 0.9),
    param("ParamMouthOpenY", 0, 0.04),
  ],
  thinking: [
    param("ParamAngleX", -0.18, 0),
    param("ParamBodyAngleX", -0.08, 0),
    param("ParamEyeOpen", 0, 0.94),
  ],
  coding: [
    param("ParamAngleX", 0.12, 0),
    param("ParamBodyAngleX", 0.08, 0),
    param("ParamMouthOpenY", 0, 0.32),
  ],
  question: [
    param("ParamAngleX", 0.04, 0),
    param("ParamBodyAngleX", -0.04, 0),
    param("ParamEyeOpen", 0, 0.98),
    param("ParamMouthOpenY", 0, 0.18),
  ],
  complete: [
    param("ParamAngleX", 0, 0),
    param("ParamBodyAngleX", 0.05, 0),
    param("ParamMouthSmile", 0, 0.85),
  ],
};

export function createPuppetStatus(
  runtimeStatus: RuntimeStatus,
  elapsedSeconds: number,
): PuppetStatus {
  const breathing = Math.sin(elapsedSeconds) * 0.18;
  return {
    runtimeStatus,
    targets: [
      ...STATUS_TARGETS[runtimeStatus],
      param("ParamBreath", 0, 0.5 + breathing),
    ],
  };
}

export function filterSupportedParams(
  params: PuppetParam[],
  supportedParams: ReadonlySet<string>,
) {
  return params.filter((entry) => supportedParams.has(entry.name));
}

export function lerpParamValue(
  current: PuppetParam,
  target: PuppetParam,
  amount: number,
): PuppetParam {
  const factor = clamp01(amount);
  return {
    name: target.name,
    x: current.x + (target.x - current.x) * factor,
    y: current.y + (target.y - current.y) * factor,
  };
}
