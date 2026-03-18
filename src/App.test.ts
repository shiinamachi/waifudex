import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, extname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { compile } from "svelte/compiler";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const tempDir = resolve(root, ".tmp-vitest-ssr");
let renderSerial = 0;

type SsrRenderer = {
  push: (chunk: string) => void;
  component: (renderComponent: (next: SsrRenderer) => void) => void;
};

async function renderSsr(relativePath: string, props: Record<string, unknown>) {
  const filePath = resolve(root, relativePath);
  const source = readFileSync(filePath, "utf8");
  const filename = basename(filePath);
  const compiled = compile(source, { filename, generate: "server" });

  mkdirSync(tempDir, { recursive: true });
  const serial = renderSerial;
  renderSerial += 1;
  const tempFile = join(
    tempDir,
    `${basename(filePath, extname(filePath))}.${process.pid}.${serial}.ssr.mjs`,
  );
  writeFileSync(tempFile, compiled.js.code, "utf8");

  try {
    const moduleUrl = `${pathToFileURL(tempFile).href}?t=${Date.now()}`;
    const module = await import(moduleUrl);

    let html = "";
    const renderer: SsrRenderer = {
      push(chunk: string) {
        html += chunk;
      },
      component(renderComponent: (next: SsrRenderer) => void) {
        html += "<!--[-->";
        renderComponent(renderer);
        html += "<!--]-->";
      },
    };

    module.default(renderer, props);
    return html;
  } finally {
    rmSync(tempFile, { force: true });
  }
}

describe("Runtime UI rendering", () => {
  it("renders bootstrap snapshot summary and detail", async () => {
    const html = await renderSsr("src/lib/components/StatusBubble.svelte", {
      loading: false,
      snapshot: {
        sessionId: "session-a",
        status: "thinking",
        summary: "Thinking through the next change",
        detail: "Codex is reviewing context and planning the next edit.",
        sessionsRoot: "/home/tester/.codex/sessions",
        source: "monitor",
        updatedAt: "2026-03-17T06:50:15.000Z",
        revision: 3,
      },
    });

    expect(html).toContain("Thinking through the next change");
    expect(html).toContain(
      "Codex is reviewing context and planning the next edit.",
    );
    expect(html).toContain("Sessions Root");
    expect(html).toContain("/home/tester/.codex/sessions");
  });

  it("renders the codex_not_installed status when the sessions root is missing", async () => {
    const html = await renderSsr("src/lib/components/StatusBubble.svelte", {
      loading: false,
      snapshot: {
        sessionId: null,
        status: "codex_not_installed",
        summary: "Codex sessions root not found",
        detail: "Waifudex could not find the configured Codex sessions directory.",
        sessionsRoot: "/home/tester/.codex/sessions",
        source: "monitor",
        updatedAt: "2026-03-18T00:00:00.000Z",
        revision: 0,
      },
    });

    expect(html).toContain("codex_not_installed");
    expect(html).toContain("Codex sessions root not found");
    expect(html).toContain(
      "Waifudex could not find the configured Codex sessions directory.",
    );
  });

  it("renders timeline entries after runtime events arrive", async () => {
    const html = await renderSsr("src/lib/components/TimelinePanel.svelte", {
      events: [
        {
          eventId: "session-a:0:0",
          sessionId: "session-a",
          sequence: 0,
          receivedAt: "2026-03-17T06:50:15.000Z",
          source: "monitor",
          kind: "session_line",
          payload: {
            rawLine: "{\"payload\":{\"type\":\"task_started\"}}",
            parsedType: "task_started",
            parseOk: true,
          },
        },
        {
          eventId: "session-a:1:1",
          sessionId: "session-a",
          sequence: 1,
          receivedAt: "2026-03-17T06:50:16.000Z",
          source: "monitor",
          kind: "session_line",
          payload: {
            rawLine: "{\"payload\":{\"type\":\"function_call\"}}",
            parsedType: "tool_call_started",
            parseOk: true,
          },
        },
      ],
    });

    expect(html).toContain("task_started");
    expect(html).toContain("tool_call_started");
    expect(html).toContain("{\"payload\":{\"type\":\"task_started\"}}");
    expect(html).toContain("{\"payload\":{\"type\":\"function_call\"}}");
  });

  it("keeps SSR rendering deterministic under parallel timeline renders", async () => {
    const renderJobs = Array.from({ length: 12 }, (_, index) =>
      renderSsr("src/lib/components/TimelinePanel.svelte", {
        events: [
          {
            eventId: `session-a:${index}:0`,
            sessionId: "session-a",
            sequence: index,
            receivedAt: "2026-03-17T06:50:15.000Z",
            source: "monitor",
            kind: "session_line",
            payload: {
              rawLine: `line-${index}`,
              parsedType: "task_started",
              parseOk: true,
            },
          },
        ],
      }),
    );

    const htmlResults = await Promise.all(renderJobs);
    for (const [index, html] of htmlResults.entries()) {
      expect(html).toContain(`line-${index}`);
      expect(html).toContain("task_started");
    }
  });

  it("shows an empty-state message when no snapshot is available yet", async () => {
    const html = await renderSsr("src/lib/components/StatusBubble.svelte", {
      loading: false,
      snapshot: null,
    });

    expect(html).toContain("No runtime snapshot yet.");
  });

  it("keeps App focused on subscribe/render without owning singleton lifecycle", () => {
    const source = readFileSync(resolve(root, "src/App.svelte"), "utf8");
    expect(source).toMatch(/store\.subscribe/);
    expect(source).not.toMatch(/store\.start\(\)/);
    expect(source).not.toMatch(/store\.stop\(\)/);
  });

  it("keeps loading tied to startup readiness instead of first subscribe callback", () => {
    const source = readFileSync(resolve(root, "src/App.svelte"), "utf8");
    expect(source).toMatch(/runtimeReady/);
    expect(source).toMatch(/runtimeReady\.finally/);
    expect(source).toMatch(/if \(!disposed\)\s*\{\s*loading = false;/);
    const subscribeMatch = source.match(
      /store\.subscribe\(\(state\) => \{([\s\S]*?)\}\);/,
    );
    expect(subscribeMatch).not.toBeNull();
    expect(subscribeMatch?.[1]).not.toMatch(/loading = false/);
  });

  it("centralizes singleton runtime store startup in main entrypoint", () => {
    const source = readFileSync(resolve(root, "src/main.ts"), "utf8");
    expect(source).toMatch(/runtimeStore\.start\(\)/);
    expect(source).toMatch(/catch\(\(error\)/);
    expect(source).toMatch(/runtimeReady/);
    expect(source).toMatch(/runtimeReady,\s*\}/);
  });

  it("creates runtime status and timeline component files", () => {
    expect(existsSync(resolve(root, "src/lib/components/StatusBubble.svelte"))).toBe(true);
    expect(existsSync(resolve(root, "src/lib/components/TimelinePanel.svelte"))).toBe(true);
  });

  it("keeps App free from the mascot presentation surface", () => {
    const source = readFileSync(resolve(root, "src/App.svelte"), "utf8");
    expect(source).not.toMatch(/Character\.svelte/);
    expect(source).not.toMatch(/waifudex:\/\/mascot-frame/);
  });

  it("keeps mascot integration out of the App shell", () => {
    expect(existsSync(resolve(root, "src/lib/mascot/types.ts"))).toBe(false);
    expect(existsSync(resolve(root, "src/lib/mascot/paramMapper.ts"))).toBe(false);
    expect(existsSync(resolve(root, "src/lib/mascot/motions"))).toBe(false);
    expect(existsSync(resolve(root, "src/lib/mascot/transport.ts"))).toBe(false);
    expect(existsSync(resolve(root, "src/lib/components/Character.svelte"))).toBe(false);

    const appSource = readFileSync(resolve(root, "src/App.svelte"), "utf8");
    expect(appSource).not.toMatch(/initMascot/);
    expect(appSource).not.toMatch(/onMascotFrame/);
    expect(appSource).not.toMatch(/disposeMascot/);
  });

  it("keeps native mascot ownership out of the Tauri command surface", () => {
    const source = readFileSync(resolve(root, "src-tauri/src/lib.rs"), "utf8");
    expect(source).toMatch(/pub mod mascot/);
    expect(source).not.toMatch(/pub mod mascot_commands/);
    expect(source).not.toMatch(/mascot_commands::init_mascot/);
    expect(source).not.toMatch(/mascot_commands::update_mascot_params/);
    expect(source).not.toMatch(/mascot_commands::resize_mascot/);
    expect(source).not.toMatch(/mascot_commands::dispose_mascot/);
  });

  it("adds the native mascot crates and build script scaffolding", () => {
    expect(existsSync(resolve(root, "scripts/build-inochi2d.sh"))).toBe(true);
    expect(existsSync(resolve(root, "crates/inochi2d-sys/Cargo.toml"))).toBe(true);
    expect(existsSync(resolve(root, "crates/inochi2d-sys/build.rs"))).toBe(true);
    expect(existsSync(resolve(root, "crates/inochi2d-sys/wrapper.h"))).toBe(true);
    expect(existsSync(resolve(root, "crates/waifudex-mascot/Cargo.toml"))).toBe(true);
    expect(existsSync(resolve(root, "crates/waifudex-mascot/src/lib.rs"))).toBe(true);
  });

  it("wires Tauri mascot management through the waifudex-mascot crate", () => {
    const cargoSource = readFileSync(resolve(root, "src-tauri/Cargo.toml"), "utf8");
    const mascotSource = readFileSync(resolve(root, "src-tauri/src/mascot.rs"), "utf8");

    expect(cargoSource).toMatch(/waifudex-mascot/);
    expect(mascotSource).toMatch(/waifudex_mascot::MascotRenderer/);
  });
});
