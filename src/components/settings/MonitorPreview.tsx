import { useEffect, useRef, useState, type PointerEvent } from "react";

import {
  characterArea,
  monitorLabel,
  monitorResolutionLabel,
  previewContainer,
} from "./monitor-preview.css";
import {
  clampPreviewCharacterOrigin,
  computeMonitorPreviewLayout,
  previewToNativePosition,
  type MonitorPreviewMonitor,
} from "./monitorPreviewLayout";
import {
  getMonitorPreviewLabel,
  getMonitorPreviewResolutionLabel,
} from "./monitorPreviewLabel";
import type { CharacterWindowPosition } from "../../hooks/useAppSetting";

const PREVIEW_WIDTH = 280;
const BASE_WIDTH = 420;
const BASE_HEIGHT = 720;

interface MonitorPreviewProps {
  monitor?: (MonitorPreviewMonitor & {
    label: string;
  }) | null;
  scale: number;
  position?: CharacterWindowPosition | null;
  onMoveCharacterWindow?: (position: CharacterWindowPosition) => void;
}

export default function MonitorPreview({
  monitor,
  scale,
  position,
  onMoveCharacterWindow,
}: MonitorPreviewProps) {
  const previewRef = useRef<HTMLDivElement | null>(null);
  const pointerIdRef = useRef<number | null>(null);
  const pointerOffsetRef = useRef({ x: 0, y: 0 });
  const [dragPosition, setDragPosition] = useState<CharacterWindowPosition | null>(
    null,
  );
  const [isDragging, setIsDragging] = useState(false);
  const activeMonitor = monitor;

  useEffect(() => {
    if (
      dragPosition &&
      position &&
      dragPosition.x === position.x &&
      dragPosition.y === position.y
    ) {
      setDragPosition(null);
    }
  }, [dragPosition, position]);

  if (!activeMonitor) {
    return null;
  }

  const resolvedMonitor = activeMonitor;

  const layout = computeMonitorPreviewLayout({
    previewWidth: PREVIEW_WIDTH,
    baseWidth: BASE_WIDTH,
    baseHeight: BASE_HEIGHT,
    scale,
    monitor: resolvedMonitor,
    position: dragPosition ?? position ?? null,
  });
  const monitorNameLabel = getMonitorPreviewLabel(resolvedMonitor.label);
  const monitorResolutionLabelText = getMonitorPreviewResolutionLabel(
    resolvedMonitor.workAreaWidth,
    resolvedMonitor.workAreaHeight,
  );

  function handlePointerDown(event: PointerEvent<HTMLDivElement>) {
    if (!previewRef.current || !onMoveCharacterWindow) {
      return;
    }

    const rect = previewRef.current.getBoundingClientRect();
    pointerIdRef.current = event.pointerId;
    pointerOffsetRef.current = {
      x: event.clientX - rect.left - layout.characterLeft,
      y: event.clientY - rect.top - layout.characterTop,
    };
    setIsDragging(true);
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent<HTMLDivElement>) {
    if (
      !previewRef.current ||
      !onMoveCharacterWindow ||
      pointerIdRef.current !== event.pointerId
    ) {
      return;
    }

    const rect = previewRef.current.getBoundingClientRect();
    const origin = clampPreviewCharacterOrigin({
      previewWidth: PREVIEW_WIDTH,
      previewHeight: layout.previewHeight,
      characterWidth: layout.characterWidth,
      characterHeight: layout.characterHeight,
      previewLeft: event.clientX - rect.left - pointerOffsetRef.current.x,
      previewTop: event.clientY - rect.top - pointerOffsetRef.current.y,
    });
    const nextPosition = previewToNativePosition({
      previewWidth: PREVIEW_WIDTH,
      baseWidth: BASE_WIDTH,
      baseHeight: BASE_HEIGHT,
      scale,
      monitor: resolvedMonitor,
      previewLeft: origin.left,
      previewTop: origin.top,
    });

    setDragPosition(nextPosition);
    onMoveCharacterWindow(nextPosition);
  }

  function finishDrag(event: PointerEvent<HTMLDivElement>) {
    if (pointerIdRef.current !== event.pointerId) {
      return;
    }

    pointerIdRef.current = null;
    setIsDragging(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  return (
    <div
      ref={previewRef}
      className={previewContainer}
      style={{ width: PREVIEW_WIDTH, height: layout.previewHeight }}
    >
      <span className={monitorResolutionLabel}>{monitorResolutionLabelText}</span>
      {monitorNameLabel ? (
        <span className={monitorLabel}>{monitorNameLabel}</span>
      ) : null}
      <div
        className={characterArea}
        onPointerCancel={finishDrag}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={finishDrag}
        style={{
          cursor: isDragging ? "grabbing" : "grab",
          left: layout.characterLeft,
          top: layout.characterTop,
          width: layout.characterWidth,
          height: layout.characterHeight,
        }}
      />
    </div>
  );
}
