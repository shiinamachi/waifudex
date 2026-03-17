<script lang="ts">
  import { CODEX_STATUS_EVENT, type CodexStatusPayload } from "../stores/codex-state";

  interface Props {
    payload: CodexStatusPayload;
    transport: "demo" | "tauri";
  }

  let { payload, transport }: Props = $props();
</script>

<aside class="panel">
  <div>
    <p class="panel__label">Event Contract</p>
    <code>{CODEX_STATUS_EVENT}</code>
  </div>
  <div>
    <p class="panel__label">Current Source</p>
    <strong>{transport}</strong>
  </div>
  <div>
    <p class="panel__label">Tracked Fields</p>
    <p>`status`, `summary`, `detail`, `updatedAt`, `source`</p>
  </div>
  <div>
    <p class="panel__label">Next Integration Step</p>
    <p>
      Replace the demo emitter with Rust-side Codex monitor events and provide a
      real Live2D model under `public/models`.
    </p>
  </div>
  <div>
    <p class="panel__label">Latest Payload</p>
    <pre>{JSON.stringify(payload, null, 2)}</pre>
  </div>
</aside>

<style>
  .panel {
    display: grid;
    gap: 14px;
    border-radius: 28px;
    background: rgba(11, 16, 28, 0.74);
    border: 1px solid rgba(255, 255, 255, 0.08);
    padding: 20px;
  }

  .panel__label {
    margin: 0 0 6px;
    color: rgba(245, 243, 239, 0.58);
    font-size: 0.76rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  strong,
  code {
    color: #ffcf99;
  }

  pre {
    margin: 0;
    overflow-x: auto;
    border-radius: 18px;
    background: rgba(7, 12, 21, 0.8);
    padding: 14px;
    font-size: 0.8rem;
    line-height: 1.5;
  }
</style>
