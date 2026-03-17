<script lang="ts">
  import type { CodexStatusMeta, CodexStatusPayload } from "../stores/codex-state";

  interface Props {
    meta: CodexStatusMeta;
    payload: CodexStatusPayload;
    updatedAtLabel: string;
  }

  let { meta, payload, updatedAtLabel }: Props = $props();
</script>

<section class="bubble" style={`--accent: ${meta.accent};`}>
  <p class="bubble__eyebrow">{meta.label}</p>
  <h2>{payload.summary}</h2>
  <p class="bubble__detail">{payload.detail}</p>
  <footer>
    <span>{payload.source}</span>
    <span>{updatedAtLabel}</span>
  </footer>
</section>

<style>
  .bubble {
    border-radius: 28px;
    background: rgba(11, 16, 28, 0.78);
    border: 1px solid rgba(255, 255, 255, 0.08);
    padding: 22px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.22);
  }

  .bubble__eyebrow {
    margin: 0;
    color: var(--accent);
    font-size: 0.78rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  h2 {
    margin: 10px 0 0;
    font-size: clamp(1.4rem, 3vw, 2rem);
    line-height: 1.04;
  }

  .bubble__detail {
    margin-top: 12px;
  }

  footer {
    margin-top: 18px;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: rgba(245, 243, 239, 0.64);
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
</style>
