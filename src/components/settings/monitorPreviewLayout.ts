import type { CharacterWindowPosition } from "../../hooks/useAppSetting";

export interface MonitorPreviewMonitor {
  workAreaLeft: number;
  workAreaTop: number;
  workAreaWidth: number;
  workAreaHeight: number;
}

interface ComputeMonitorPreviewLayoutInput {
  previewWidth: number;
  baseWidth: number;
  baseHeight: number;
  scale: number;
  monitor: MonitorPreviewMonitor;
  position: CharacterWindowPosition | null;
}

export interface MonitorPreviewLayout {
  previewHeight: number;
  characterLeft: number;
  characterTop: number;
  characterWidth: number;
  characterHeight: number;
}

export function computeMonitorPreviewLayout({
  previewWidth,
  baseWidth,
  baseHeight,
  scale,
  monitor,
  position,
}: ComputeMonitorPreviewLayoutInput): MonitorPreviewLayout {
  const previewScale = previewWidth / monitor.workAreaWidth;
  const previewHeight = monitor.workAreaHeight * previewScale;
  const characterWidth = baseWidth * scale * previewScale;
  const characterHeight = baseHeight * scale * previewScale;
  const maxLeft = monitor.workAreaLeft + Math.max(0, monitor.workAreaWidth - baseWidth * scale);
  const maxTop = monitor.workAreaTop + Math.max(0, monitor.workAreaHeight - baseHeight * scale);
  const fallbackX =
    monitor.workAreaLeft + Math.max(0, (monitor.workAreaWidth - baseWidth * scale) / 2);
  const fallbackY =
    monitor.workAreaTop + Math.max(0, (monitor.workAreaHeight - baseHeight * scale) / 2);
  const left = clamp(position?.x ?? fallbackX, monitor.workAreaLeft, maxLeft);
  const top = clamp(position?.y ?? fallbackY, monitor.workAreaTop, maxTop);

  return {
    previewHeight,
    characterLeft: (left - monitor.workAreaLeft) * previewScale,
    characterTop: (top - monitor.workAreaTop) * previewScale,
    characterWidth,
    characterHeight,
  };
}

interface PreviewToNativePositionInput {
  previewWidth: number;
  baseWidth: number;
  baseHeight: number;
  scale: number;
  monitor: MonitorPreviewMonitor;
  previewLeft: number;
  previewTop: number;
}

export function previewToNativePosition({
  previewWidth,
  baseWidth,
  baseHeight,
  scale,
  monitor,
  previewLeft,
  previewTop,
}: PreviewToNativePositionInput): CharacterWindowPosition {
  const previewScale = previewWidth / monitor.workAreaWidth;
  const previewHeight = monitor.workAreaHeight * previewScale;
  const characterWidth = baseWidth * scale * previewScale;
  const characterHeight = baseHeight * scale * previewScale;
  const clampedOrigin = clampPreviewCharacterOrigin({
    previewWidth,
    previewHeight,
    characterWidth,
    characterHeight,
    previewLeft,
    previewTop,
  });

  return {
    x: Math.round(monitor.workAreaLeft + clampedOrigin.left / previewScale),
    y: Math.round(monitor.workAreaTop + clampedOrigin.top / previewScale),
  };
}

interface ClampPreviewCharacterOriginInput {
  previewWidth: number;
  previewHeight: number;
  characterWidth: number;
  characterHeight: number;
  previewLeft: number;
  previewTop: number;
}

export function clampPreviewCharacterOrigin({
  previewWidth,
  previewHeight,
  characterWidth,
  characterHeight,
  previewLeft,
  previewTop,
}: ClampPreviewCharacterOriginInput): { left: number; top: number } {
  return {
    left: clamp(previewLeft, 0, Math.max(0, previewWidth - characterWidth)),
    top: clamp(previewTop, 0, Math.max(0, previewHeight - characterHeight)),
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
