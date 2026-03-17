import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import {
  RUNTIME_EVENT_STREAM,
  RUNTIME_SNAPSHOT_EVENT,
  type RuntimeBootstrap,
  type RuntimeEvent,
  type RuntimeSnapshot,
} from "../contracts/generated/runtime";

type SnapshotListener = (snapshot: RuntimeSnapshot) => void;
type EventListener = (event: RuntimeEvent) => void;

export async function getRuntimeBootstrap(): Promise<RuntimeBootstrap> {
  return invoke<RuntimeBootstrap>("get_runtime_bootstrap");
}

export async function listenRuntimeSnapshot(
  listener: SnapshotListener,
): Promise<UnlistenFn> {
  return listen<RuntimeSnapshot>(RUNTIME_SNAPSHOT_EVENT, (event) => {
    listener(event.payload);
  });
}

export async function listenRuntimeEvent(
  listener: EventListener,
): Promise<UnlistenFn> {
  return listen<RuntimeEvent>(RUNTIME_EVENT_STREAM, (event) => {
    listener(event.payload);
  });
}
