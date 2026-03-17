<script lang="ts">
  import type { RuntimeEvent } from "../contracts/generated/runtime";

  let { events = [] }: { events?: RuntimeEvent[] } = $props();
</script>

<section class="timeline-panel" aria-live="polite">
  <header class="timeline-header">
    <h2>Live Timeline</h2>
    <p>{events.length} event{events.length === 1 ? "" : "s"}</p>
  </header>

  {#if events.length === 0}
    <p class="timeline-empty">No timeline events yet.</p>
  {:else}
    <ol class="timeline-list">
      {#each events as event (event.eventId)}
        <li class="timeline-item">
          <div class="timeline-meta">
            <span class="timeline-type">{event.payload.parsedType ?? "unknown"}</span>
            <span class="timeline-seq">#{event.sequence}</span>
          </div>
          <pre class="timeline-raw">{event.payload.rawLine}</pre>
        </li>
      {/each}
    </ol>
  {/if}
</section>
