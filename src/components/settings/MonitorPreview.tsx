import { useEffect, useState } from "react";
import { currentMonitor } from "@tauri-apps/api/window";

import {
  characterArea,
  monitorLabel,
  monitorResolutionLabel,
  previewContainer,
} from "./monitor-preview.css";
import {
  getMonitorPreviewLabel,
  getMonitorPreviewResolutionLabel,
} from "./monitorPreviewLabel";

const PREVIEW_WIDTH = 280;
const BASE_WIDTH = 420;
const BASE_HEIGHT = 720;

interface MonitorPreviewProps {
  monitorName?: string | null;
  scale: number;
}

export default function MonitorPreview({
  monitorName,
  scale,
}: MonitorPreviewProps) {
  const [monitorSize, setMonitorSize] = useState<{
    width: number;
    height: number;
  } | null>(null);

  useEffect(() => {
    currentMonitor().then((monitor) => {
      if (monitor) {
        setMonitorSize({
          width: monitor.size.width,
          height: monitor.size.height,
        });
      }
    });
  }, []);

  if (!monitorSize) {
    return null;
  }

  const previewScale = PREVIEW_WIDTH / monitorSize.width;
  const previewHeight = monitorSize.height * previewScale;
  const charWidth = BASE_WIDTH * scale * previewScale;
  const charHeight = BASE_HEIGHT * scale * previewScale;
  const monitorNameLabel = getMonitorPreviewLabel(monitorName);
  const monitorResolutionLabelText = getMonitorPreviewResolutionLabel(
    monitorSize.width,
    monitorSize.height,
  );

  return (
    <div
      className={previewContainer}
      style={{ width: PREVIEW_WIDTH, height: previewHeight }}
    >
      <span className={monitorResolutionLabel}>{monitorResolutionLabelText}</span>
      {monitorNameLabel ? (
        <span className={monitorLabel}>{monitorNameLabel}</span>
      ) : null}
      <div
        className={characterArea}
        style={{ width: charWidth, height: charHeight }}
      />
    </div>
  );
}
