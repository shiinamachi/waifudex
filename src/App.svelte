<script lang="ts">
  import { onMount } from "svelte";
  import Character from "./lib/components/Character.svelte";
  import StatusBubble from "./lib/components/StatusBubble.svelte";
  import StatusPanel from "./lib/components/StatusPanel.svelte";
  import { codexStore } from "./lib/stores/codexStore.svelte";

  const title = "Waifudex";

  onMount(() => {
    void codexStore.start();

    return () => {
      codexStore.destroy();
    };
  });
</script>

<svelte:head>
  <title>{title}</title>
</svelte:head>

<main class="app-shell">
  <section class="app-shell__copy">
    <div class="app-shell__header" data-tauri-drag-region>
      <p class="app-shell__eyebrow">Desktop Mascot Boilerplate</p>
      <h1>{title}</h1>
      <p>
        Tauri event skeleton, Svelte runes state, Pixi placeholder surface, and
        Rust module boundaries are ready for Codex tracking.
      </p>
    </div>
    <StatusBubble
      meta={codexStore.meta}
      payload={codexStore.payload}
      updatedAtLabel={codexStore.updatedAtLabel}
    />
    <button class="app-shell__toggle" type="button" on:click={() => codexStore.togglePanel()}>
      {codexStore.isPanelOpen ? "Hide status contract" : "Show status contract"}
    </button>
    {#if codexStore.isPanelOpen}
      <StatusPanel payload={codexStore.payload} transport={codexStore.transport} />
    {/if}
  </section>

  <section class="app-shell__figure">
    <Character payload={codexStore.payload} meta={codexStore.meta} />
  </section>
</main>
