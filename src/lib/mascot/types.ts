import type { RuntimeStatus } from "../contracts/generated/runtime";

export type PuppetParam = {
  name: string;
  x: number;
  y: number;
};

export type PuppetStatus = {
  runtimeStatus: RuntimeStatus;
  targets: PuppetParam[];
};

export type MascotFrame = {
  width: number;
  height: number;
  rgba: Uint8ClampedArray;
  revision: number;
};
