import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { MascotFrame, PuppetParam } from "./types";

export const MASCOT_FRAME_EVENT = "waifudex://mascot-frame" as const;

type MascotFramePayload = {
  width: number;
  height: number;
  rgba: number[];
  revision: number;
};

type InitMascotRequest = {
  modelPath: string;
  width: number;
  height: number;
};

type ResizeMascotRequest = {
  width: number;
  height: number;
};

type UpdateMascotParamsRequest = {
  params: PuppetParam[];
};

export async function initMascot(
  width: number,
  height: number,
  modelPath = "/models/Aka.inx",
): Promise<string[]> {
  return invoke<string[]>("init_mascot", {
    modelPath,
    width,
    height,
  } satisfies InitMascotRequest);
}

export async function updateMascotParams(params: PuppetParam[]): Promise<void> {
  await invoke<void>("update_mascot_params", {
    params,
  } satisfies UpdateMascotParamsRequest);
}

export async function resizeMascot(width: number, height: number): Promise<void> {
  await invoke<void>("resize_mascot", {
    width,
    height,
  } satisfies ResizeMascotRequest);
}

export async function disposeMascot(): Promise<void> {
  await invoke<void>("dispose_mascot");
}

export async function onMascotFrame(
  listener: (frame: MascotFrame) => void,
): Promise<UnlistenFn> {
  return listen<MascotFramePayload>(MASCOT_FRAME_EVENT, (event) => {
    listener({
      width: event.payload.width,
      height: event.payload.height,
      rgba: new Uint8ClampedArray(event.payload.rgba),
      revision: event.payload.revision,
    });
  });
}
