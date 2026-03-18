<script lang="ts">
  import { onMount } from "svelte";

  import type { RuntimeSnapshot } from "./lib/contracts/generated/runtime";
  import Character from "./lib/components/Character.svelte";
  import type { RuntimeStoreState } from "./lib/stores/runtimeStore.svelte";

  type RuntimeStore = {
    subscribe: (listener: (state: RuntimeStoreState) => void) => () => void;
  };

  let {
    store,
    runtimeReady,
  }: { store: RuntimeStore; runtimeReady: Promise<unknown> } = $props();

  let snapshot = $state<RuntimeSnapshot | null>(null);

  onMount(() => {
    const unsubscribe = store.subscribe((state) => {
      snapshot = state.snapshot;
    });

    return () => {
      unsubscribe();
    };
  });
</script>

<Character status={snapshot?.status ?? "idle"} />
