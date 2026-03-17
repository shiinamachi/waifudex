<script lang="ts">
  import { onMount } from "svelte";
  import { Application } from "pixi.js";
  import type { CodexStatusMeta, CodexStatusPayload } from "../stores/codex-state";

  interface Props {
    payload: CodexStatusPayload;
    meta: CodexStatusMeta;
  }

  let { payload, meta }: Props = $props();
  let canvasHost = $state<HTMLDivElement | null>(null);

  onMount(() => {
    let disposed = false;
    const app = new Application();

    void (async () => {
      if (!canvasHost) {
        return;
      }

      await app.init({
        antialias: true,
        backgroundAlpha: 0,
        resizeTo: canvasHost,
      });

      if (disposed || !canvasHost) {
        app.destroy(true);
        return;
      }

      app.canvas.classList.add("character__canvas");
      canvasHost.appendChild(app.canvas);
    })();

    return () => {
      disposed = true;
      app.destroy(true);
    };
  });
</script>

<div class="character" style={`--accent: ${meta.accent};`}>
  <div bind:this={canvasHost} class="character__surface"></div>
  <div class="character__silhouette"></div>
  <div class="character__badge">
    <span>Pixi Placeholder</span>
    <strong>{payload.status}</strong>
  </div>
  <p class="character__note">
    Live2D model hookup is deferred until compatible runtime assets are added.
  </p>
</div>

<style>
  .character {
    position: relative;
    aspect-ratio: 1 / 1.22;
    min-height: 360px;
    border-radius: 32px;
    overflow: hidden;
    background:
      radial-gradient(circle at top, rgba(255, 226, 204, 0.38), transparent 42%),
      linear-gradient(180deg, rgba(16, 25, 41, 0.6), rgba(11, 18, 29, 0.92));
    border: 1px solid rgba(255, 255, 255, 0.12);
  }

  .character__surface {
    position: absolute;
    inset: 0;
  }

  .character__surface :global(canvas.character__canvas) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .character__silhouette {
    position: absolute;
    inset: 14% 18% 12%;
    border-radius: 46% 46% 36% 36%;
    background:
      radial-gradient(circle at 50% 18%, color-mix(in srgb, var(--accent) 70%, white), transparent 32%),
      linear-gradient(180deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.02));
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.14),
      0 25px 60px color-mix(in srgb, var(--accent) 28%, transparent);
    backdrop-filter: blur(12px);
  }

  .character__badge {
    position: absolute;
    left: 16px;
    right: 16px;
    bottom: 20px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 16px;
    border-radius: 18px;
    background: rgba(9, 13, 24, 0.72);
    border: 1px solid rgba(255, 255, 255, 0.08);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.74rem;
  }

  .character__badge strong {
    color: var(--accent);
    font-size: 0.78rem;
  }

  .character__note {
    position: absolute;
    top: 18px;
    left: 16px;
    width: min(220px, calc(100% - 32px));
    margin: 0;
    padding: 12px 14px;
    border-radius: 18px;
    background: rgba(7, 12, 21, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 0.82rem;
  }
</style>
