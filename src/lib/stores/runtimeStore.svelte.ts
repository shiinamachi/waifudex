import type {
  RuntimeBootstrap,
  RuntimeEvent,
  RuntimeSnapshot,
} from "../contracts/generated/runtime";

type RuntimeStoreListener = (state: RuntimeStoreState) => void;
type Unlisten = () => void | Promise<void>;

export type RuntimeStoreState = {
  snapshot: RuntimeSnapshot | null;
  timeline: RuntimeEvent[];
};

export type RuntimeTransport = {
  getRuntimeBootstrap: () => Promise<RuntimeBootstrap>;
  listenRuntimeSnapshot: (listener: (snapshot: RuntimeSnapshot) => void) => Promise<Unlisten>;
  listenRuntimeEvent: (listener: (event: RuntimeEvent) => void) => Promise<Unlisten>;
};

export function createRuntimeStore(transport: RuntimeTransport) {
  let state: RuntimeStoreState = {
    snapshot: null,
    timeline: [],
  };

  let started = false;
  let unlisten: Unlisten[] = [];
  const listeners = new Set<RuntimeStoreListener>();

  const notify = () => {
    for (const listener of listeners) {
      listener(state);
    }
  };

  const updateSnapshot = (incoming: RuntimeSnapshot) => {
    const currentRevision = state.snapshot?.revision ?? -1;
    if (incoming.revision <= currentRevision) {
      return;
    }

    state = { ...state, snapshot: incoming };
    notify();
  };

  const applyBootstrap = (bootstrap: RuntimeBootstrap) => {
    if (bootstrap.snapshot === null) {
      return;
    }
    updateSnapshot(bootstrap.snapshot);
  };

  const appendTimeline = (event: RuntimeEvent) => {
    state = {
      ...state,
      timeline: [...state.timeline, event],
    };
    notify();
  };

  const stopUnlisten = async (callbacks: Unlisten[]) => {
    await Promise.all(callbacks.map((callback) => Promise.resolve(callback())));
  };

  return {
    subscribe(listener: RuntimeStoreListener) {
      listeners.add(listener);
      listener(state);

      return () => {
        listeners.delete(listener);
      };
    },

    getState(): RuntimeStoreState {
      return state;
    },

    async start() {
      if (started) {
        return;
      }

      started = true;
      const callbacks: Unlisten[] = [];

      try {
        callbacks.push(
          await transport.listenRuntimeSnapshot((snapshot) => {
            updateSnapshot(snapshot);
          }),
        );
        callbacks.push(
          await transport.listenRuntimeEvent((event) => {
            appendTimeline(event);
          }),
        );

        unlisten = callbacks;
        const bootstrap = await transport.getRuntimeBootstrap();
        applyBootstrap(bootstrap);
      } catch (error) {
        await stopUnlisten(callbacks);
        unlisten = [];
        started = false;
        throw error;
      }
    },

    async stop() {
      if (!started) {
        return;
      }

      started = false;
      const callbacks = unlisten;
      unlisten = [];
      await stopUnlisten(callbacks);
    },
  };
}
