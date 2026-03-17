import {
  CODEX_STATUS_EVENT,
  codexStatusOrder,
  createCodexStatusPayload,
  createDemoSequence,
  getCodexStatusMeta,
} from "./codex-state";

describe("codex-state", () => {
  it("exposes the status event contract and ordered demo states", () => {
    expect(CODEX_STATUS_EVENT).toBe("waifudex://codex-status");
    expect(codexStatusOrder).toEqual([
      "idle",
      "thinking",
      "writing",
      "running_tests",
      "success",
      "error",
    ]);
  });

  it("creates payloads from metadata defaults", () => {
    const payload = createCodexStatusPayload("thinking");

    expect(payload.status).toBe("thinking");
    expect(payload.source).toBe("demo");
    expect(payload.summary).toContain("Thinking");
    expect(payload.detail).toContain("Codex");
    expect(payload.updatedAt).toMatch(/T/);
  });

  it("builds a demo sequence that covers all visible states", () => {
    const sequence = createDemoSequence("2026-03-17T06:50:00.000Z");
    const runningTestsMeta = getCodexStatusMeta("running_tests");

    expect(sequence).toHaveLength(codexStatusOrder.length);
    expect(sequence[3]).toMatchObject({
      status: "running_tests",
      summary: runningTestsMeta.summary,
    });
    expect(sequence.at(-1)?.status).toBe("error");
  });
});
