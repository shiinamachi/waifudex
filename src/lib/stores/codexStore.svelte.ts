import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  CODEX_STATUS_EVENT,
  createCodexStatusPayload,
  createDemoSequence,
  getCodexStatusMeta,
  type CodexStatusPayload,
} from "./codex-state";

class CodexStore {
  payload = $state<CodexStatusPayload>(createCodexStatusPayload("idle"));
  isPanelOpen = $state(false);
  transport = $state<"demo" | "tauri">("demo");

  #demoTimer: number | null = null;
  #unsubscribe: UnlistenFn | null = null;

  get meta() {
    return getCodexStatusMeta(this.payload.status);
  }

  get updatedAtLabel() {
    return new Date(this.payload.updatedAt).toLocaleTimeString();
  }

  async start() {
    if (typeof window === "undefined") {
      return;
    }

    if (this.#unsubscribe || this.#demoTimer !== null) {
      return;
    }

    try {
      const { listen } = await import("@tauri-apps/api/event");

      this.#unsubscribe = await listen<CodexStatusPayload>(
        CODEX_STATUS_EVENT,
        (event) => {
          this.transport = "tauri";
          this.applyPayload({
            ...event.payload,
            source: "backend",
          });
          this.stopDemo();
        },
      );
    } catch {
      this.startDemo();
    }
  }

  togglePanel() {
    this.isPanelOpen = !this.isPanelOpen;
  }

  closePanel() {
    this.isPanelOpen = false;
  }

  destroy() {
    this.stopDemo();

    if (this.#unsubscribe) {
      this.#unsubscribe();
      this.#unsubscribe = null;
    }
  }

  private applyPayload(payload: CodexStatusPayload) {
    this.payload = payload;
  }

  private startDemo() {
    const sequence = createDemoSequence();
    let index = 0;

    this.transport = "demo";
    this.applyPayload(sequence[index]!);
    this.#demoTimer = window.setInterval(() => {
      index = (index + 1) % sequence.length;
      this.applyPayload(sequence[index]!);
    }, 2400);
  }

  private stopDemo() {
    if (this.#demoTimer !== null) {
      window.clearInterval(this.#demoTimer);
      this.#demoTimer = null;
    }
  }
}

export const codexStore = new CodexStore();
