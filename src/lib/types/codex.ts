export const codexStatusOrder = [
  "idle",
  "thinking",
  "writing",
  "running_tests",
  "success",
  "error",
] as const;

export type CodexStatus = (typeof codexStatusOrder)[number];

export type CodexPayloadSource = "demo" | "backend";

export interface CodexStatusMeta {
  label: string;
  summary: string;
  detail: string;
  accent: string;
}

export interface CodexStatusPayload {
  status: CodexStatus;
  summary: string;
  detail: string;
  updatedAt: string;
  source: CodexPayloadSource;
}
