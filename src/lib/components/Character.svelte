<script lang="ts">
  import { onMount } from "svelte";

  import type { RuntimeStatus } from "../contracts/generated/runtime";
  import {
    createPuppetStatus,
    filterSupportedParams,
    lerpParamValue,
  } from "../mascot/paramMapper";
  import {
    disposeMascot,
    initMascot,
    onMascotFrame,
    resizeMascot,
    updateMascotParams,
  } from "../mascot/transport";
  import type { MascotFrame, PuppetParam } from "../mascot/types";

  let { status }: { status: RuntimeStatus } = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let errorMessage = $state<string | null>(null);

  const currentParams = new Map<string, PuppetParam>();

  onMount(() => {
    if (canvas === null) {
      return;
    }

    const stageCanvas = canvas;
    const context = stageCanvas.getContext("2d");

    if (context === null) {
      errorMessage = "failed to initialize 2D canvas context";
      return;
    }

    let disposed = false;
    let animationFrame = 0;
    let lastFrame = 0;
    let startedAt = 0;
    let resizeObserver: ResizeObserver | null = null;
    let unlistenFrame: (() => void) | null = null;
    let supportedParams = new Set<string>();

    const measureCanvas = () => {
      const dpr = window.devicePixelRatio || 1;
      return {
        width: Math.max(1, Math.round(stageCanvas.clientWidth * dpr)),
        height: Math.max(1, Math.round(stageCanvas.clientHeight * dpr)),
      };
    };

    const paintFrame = (frame: MascotFrame) => {
      if (stageCanvas.width !== frame.width || stageCanvas.height !== frame.height) {
        stageCanvas.width = frame.width;
        stageCanvas.height = frame.height;
      }

      context.putImageData(
        new ImageData(Uint8ClampedArray.from(frame.rgba), frame.width, frame.height),
        0,
        0,
      );
    };

    const tick = (timestamp: number) => {
      if (disposed) {
        return;
      }

      if (startedAt === 0) {
        startedAt = timestamp;
      }

      const dt = lastFrame === 0 ? 1 / 60 : (timestamp - lastFrame) / 1000;
      lastFrame = timestamp;

      const elapsedSeconds = (timestamp - startedAt) / 1000;
      const targetStatus = createPuppetStatus(status, elapsedSeconds);
      const targets = filterSupportedParams(targetStatus.targets, supportedParams);

      for (const target of targets) {
        const current = currentParams.get(target.name) ?? target;
        const next = lerpParamValue(current, target, 0.14);
        currentParams.set(target.name, next);
      }

      void updateMascotParams(Array.from(currentParams.values())).catch((error) => {
        if (!disposed) {
          errorMessage =
            error instanceof Error ? error.message : "failed to update mascot parameters";
        }
      });

      animationFrame = requestAnimationFrame((nextTimestamp) => {
        tick(nextTimestamp);
      });
    };

    const { width, height } = measureCanvas();

    void Promise.all([
      initMascot(width, height),
      onMascotFrame((frame) => {
        paintFrame(frame);
      }),
    ])
      .then(([availableParams, unlisten]) => {
        if (disposed) {
          void unlisten();
          void disposeMascot();
          return;
        }

        supportedParams = new Set(availableParams);
        unlistenFrame = unlisten;
        resizeObserver = new ResizeObserver(() => {
          const nextSize = measureCanvas();
          void resizeMascot(nextSize.width, nextSize.height).catch((error) => {
            if (!disposed) {
              errorMessage =
                error instanceof Error ? error.message : "failed to resize mascot renderer";
            }
          });
        });
        resizeObserver.observe(stageCanvas);
        animationFrame = requestAnimationFrame((timestamp) => {
          tick(timestamp);
        });
      })
      .catch((error) => {
        errorMessage = error instanceof Error ? error.message : "failed to load mascot model";
      });

    return () => {
      disposed = true;
      cancelAnimationFrame(animationFrame);
      resizeObserver?.disconnect();
      if (unlistenFrame !== null) {
        void unlistenFrame();
      }
      void disposeMascot();
    };
  });
</script>

<div class="character-wrapper">
  <canvas bind:this={canvas} class="character-canvas"></canvas>
  <div class="drag-region" data-tauri-drag-region></div>
</div>
