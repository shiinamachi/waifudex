<script lang="ts">
  import type { RuntimeSnapshot } from "../contracts/generated/runtime";

  let {
    loading = false,
    snapshot = null,
  }: { loading?: boolean; snapshot?: RuntimeSnapshot | null } = $props();
</script>

<section class="status-bubble" aria-live="polite">
  <header class="status-header">
    <p class="status-eyebrow">Current Snapshot</p>
    {#if snapshot !== null}
      <p class="status-kind">{snapshot.status}</p>
    {/if}
  </header>

  {#if loading}
    <p class="status-empty">Loading runtime snapshot...</p>
  {:else if snapshot === null}
    <p class="status-empty">No runtime snapshot yet.</p>
  {:else}
    <h2 class="status-summary">{snapshot.summary}</h2>
    <p class="status-detail">{snapshot.detail}</p>
    <dl class="status-meta">
      <div>
        <dt>Session</dt>
        <dd>{snapshot.sessionId ?? "none"}</dd>
      </div>
      <div>
        <dt>Revision</dt>
        <dd>{snapshot.revision}</dd>
      </div>
      <div>
        <dt>Sessions Root</dt>
        <dd>{snapshot.sessionsRoot}</dd>
      </div>
      <div>
        <dt>Updated</dt>
        <dd>{snapshot.updatedAt}</dd>
      </div>
      <div>
        <dt>Source</dt>
        <dd>{snapshot.source}</dd>
      </div>
    </dl>
  {/if}
</section>
