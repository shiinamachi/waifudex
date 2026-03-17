import {
  codexStatusOrder,
  type CodexPayloadSource,
  type CodexStatus,
  type CodexStatusMeta,
  type CodexStatusPayload,
} from "../types/codex";

export { codexStatusOrder } from "../types/codex";
export type {
  CodexPayloadSource,
  CodexStatus,
  CodexStatusMeta,
  CodexStatusPayload,
} from "../types/codex";

export const CODEX_STATUS_EVENT = "waifudex://codex-status";

const codexStatusMeta: Record<CodexStatus, CodexStatusMeta> = {
  idle: {
    label: "Idle",
    summary: "Waiting for the next Codex task",
    detail: "Codex is connected and holding a calm default posture.",
    accent: "#ffcf99",
  },
  thinking: {
    label: "Thinking",
    summary: "Thinking through the next change",
    detail: "Codex is analyzing the workspace and planning the next mutation.",
    accent: "#ffd36a",
  },
  writing: {
    label: "Writing Code",
    summary: "Writing implementation details",
    detail: "Codex is editing the project and syncing source files.",
    accent: "#7ce1c3",
  },
  running_tests: {
    label: "Running Tests",
    summary: "Running tests and build checks",
    detail: "Codex is validating the latest changes before moving on.",
    accent: "#8cc8ff",
  },
  success: {
    label: "Success",
    summary: "Latest task completed cleanly",
    detail: "Codex finished the active step and the mascot can celebrate.",
    accent: "#9fffad",
  },
  error: {
    label: "Error",
    summary: "A blocking issue needs attention",
    detail: "Codex hit an error and is waiting for correction or retry.",
    accent: "#ff8c7c",
  },
};

export function getCodexStatusMeta(status: CodexStatus): CodexStatusMeta {
  return codexStatusMeta[status];
}

export function createCodexStatusPayload(
  status: CodexStatus,
  source: CodexPayloadSource = "demo",
  updatedAt = new Date().toISOString(),
): CodexStatusPayload {
  const meta = getCodexStatusMeta(status);

  return {
    status,
    summary: meta.summary,
    detail: meta.detail,
    updatedAt,
    source,
  };
}

export function createDemoSequence(seed = new Date().toISOString()): CodexStatusPayload[] {
  const baseTime = new Date(seed).getTime();

  return codexStatusOrder.map((status, index) => {
    const updatedAt = new Date(baseTime + index * 15_000).toISOString();
    return createCodexStatusPayload(status, "demo", updatedAt);
  });
}
