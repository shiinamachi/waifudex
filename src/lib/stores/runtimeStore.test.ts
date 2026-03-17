import { describe, expect, it, vi } from "vitest";

import type {
  RuntimeBootstrap,
  RuntimeEvent,
  RuntimeSnapshot,
} from "../contracts/generated/runtime";
import { createRuntimeStore, type RuntimeTransport } from "./runtimeStore.svelte";

type SnapshotListener = (snapshot: RuntimeSnapshot) => void;
type EventListener = (event: RuntimeEvent) => void;
type Deferred<T> = {
  promise: Promise<T>;
  resolve: (value: T) => void;
};

function createSnapshot(revision: number): RuntimeSnapshot {
  return {
    sessionId: "session-a",
    status: "thinking",
    summary: `summary-${revision}`,
    detail: `detail-${revision}`,
    sessionsRoot: "/home/tester/.codex/sessions",
    source: "monitor",
    updatedAt: "2026-03-17T06:50:15.000Z",
    revision,
  };
}

function createTimelineEvent(sequence: number): RuntimeEvent {
  return {
    eventId: `session-a:${sequence}:0`,
    sessionId: "session-a",
    sequence,
    receivedAt: "2026-03-17T06:50:15.000Z",
    source: "monitor",
    kind: "session_line",
    payload: {
      rawLine: `line-${sequence}`,
      parsedType: "task_started",
      parseOk: true,
    },
  };
}

function createDeferred<T>(): Deferred<T> {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function createMockTransport(bootstrap: RuntimeBootstrap) {
  let snapshotListener: SnapshotListener | null = null;
  let eventListener: EventListener | null = null;

  const transport: RuntimeTransport = {
    getRuntimeBootstrap: vi.fn(async () => bootstrap),
    listenRuntimeSnapshot: vi.fn(async (listener: SnapshotListener) => {
      snapshotListener = listener;
      return () => {};
    }),
    listenRuntimeEvent: vi.fn(async (listener: EventListener) => {
      eventListener = listener;
      return () => {};
    }),
  };

  return {
    transport,
    emitSnapshot(snapshot: RuntimeSnapshot) {
      if (snapshotListener === null) {
        throw new Error("snapshot listener not registered");
      }
      snapshotListener(snapshot);
    },
    emitTimeline(event: RuntimeEvent) {
      if (eventListener === null) {
        throw new Error("timeline listener not registered");
      }
      eventListener(event);
    },
  };
}

describe("runtimeStore", () => {
  it("populates the current snapshot from bootstrap", async () => {
    const bootstrapSnapshot = createSnapshot(2);
    const { transport } = createMockTransport({ snapshot: bootstrapSnapshot });
    const store = createRuntimeStore(transport);

    await store.start();

    expect(store.getState().snapshot).toEqual(bootstrapSnapshot);
    expect(store.getState().timeline).toEqual([]);
  });

  it("applies runtime snapshot updates only when revision is newer", async () => {
    const bootstrapSnapshot = createSnapshot(3);
    const staleSnapshot = createSnapshot(1);
    const newerSnapshot = createSnapshot(5);

    const mock = createMockTransport({ snapshot: bootstrapSnapshot });
    const store = createRuntimeStore(mock.transport);
    await store.start();

    mock.emitSnapshot(staleSnapshot);
    expect(store.getState().snapshot).toEqual(bootstrapSnapshot);

    mock.emitSnapshot(newerSnapshot);
    expect(store.getState().snapshot).toEqual(newerSnapshot);
  });

  it("appends runtime timeline events in received order", async () => {
    const mock = createMockTransport({ snapshot: null });
    const store = createRuntimeStore(mock.transport);
    await store.start();

    const first = createTimelineEvent(0);
    const second = createTimelineEvent(1);

    mock.emitTimeline(first);
    mock.emitTimeline(second);

    expect(store.getState().timeline).toEqual([first, second]);
  });

  it("does not clear a live snapshot when bootstrap later returns null", async () => {
    const deferredBootstrap = createDeferred<RuntimeBootstrap>();
    let snapshotListener: SnapshotListener | null = null;
    let eventListener: EventListener | null = null;

    const transport: RuntimeTransport = {
      getRuntimeBootstrap: vi.fn(async () => deferredBootstrap.promise),
      listenRuntimeSnapshot: vi.fn(async (listener: SnapshotListener) => {
        snapshotListener = listener;
        return () => {};
      }),
      listenRuntimeEvent: vi.fn(async (listener: EventListener) => {
        eventListener = listener;
        return () => {};
      }),
    };

    const store = createRuntimeStore(transport);
    const startPromise = store.start();
    await Promise.resolve();

    expect(eventListener).not.toBeNull();
    expect(snapshotListener).not.toBeNull();
    const activeSnapshotListener = snapshotListener as
      | SnapshotListener
      | null;
    if (typeof activeSnapshotListener !== "function") {
      throw new Error("snapshot listener was not registered");
    }
    activeSnapshotListener(createSnapshot(7));

    deferredBootstrap.resolve({ snapshot: null });
    await startPromise;

    expect(store.getState().snapshot).toEqual(createSnapshot(7));
  });
});
