<script lang="ts">
  import { onMount } from "svelte";

  import type { RuntimeEvent, RuntimeSnapshot } from "./lib/contracts/generated/runtime";
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

    runtimeReady.finally(() => {
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
  <StatusBubble {loading} {snapshot} />
  <TimelinePanel events={timeline} />
</main>
