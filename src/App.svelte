<script lang="ts">
  import { onMount } from "svelte";

  import type {
    RuntimeEvent,
    RuntimeSnapshot,
  } from "./lib/contracts/generated/runtime";
  import Character from "./lib/components/Character.svelte";
  import StatusBubble from "./lib/components/StatusBubble.svelte";
  import TimelinePanel from "./lib/components/TimelinePanel.svelte";
  import type { RuntimeStoreState } from "./lib/stores/runtimeStore.svelte";

  type RuntimeStore = {
    subscribe: (listener: (state: RuntimeStoreState) => void) => () => void;
  };

  let {
    store,
    runtimeReady,
  }: { store: RuntimeStore; runtimeReady: Promise<unknown> } = $props();

  let loading = $state(true);
  let snapshot = $state<RuntimeSnapshot | null>(null);
  let timeline = $state<RuntimeEvent[]>([]);

  onMount(() => {
    let disposed = false;
    const unsubscribe = store.subscribe((state) => {
      snapshot = state.snapshot;
      timeline = state.timeline;
    });

    void runtimeReady.finally(() => {
      if (!disposed) {
        loading = false;
      }
    });

    return () => {
      disposed = true;
      unsubscribe();
    };
  });
</script>

<main class="app-shell">
  <header class="app-header">
    <p class="app-eyebrow">Waifudex Runtime</p>
    <h1>Codex Status Debug View</h1>
    <p class="app-caption">Snapshot and live event stream from Tauri monitor.</p>
  </header>

  <div class="app-grid">
    <Character status={snapshot?.status ?? "idle"} />
    <div class="app-sidebar">
      <StatusBubble {loading} {snapshot} />
      <TimelinePanel events={timeline} />
    </div>
  </div>
</main>
