import { useEffect, useState } from "react";
import { currentMonitor } from "@tauri-apps/api/window";

import { characterArea, monitorLabel, previewContainer } from "./monitor-preview.css";

const PREVIEW_WIDTH = 280;
const BASE_WIDTH = 420;
const BASE_HEIGHT = 720;

interface MonitorPreviewProps {
  scale: number;
}

export default function MonitorPreview({ scale }: MonitorPreviewProps) {
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

  return (
    <div
      className={previewContainer}
      style={{ width: PREVIEW_WIDTH, height: previewHeight }}
    >
      <span className={monitorLabel}>
        {monitorSize.width} x {monitorSize.height}
      </span>
      <div
        className={characterArea}
        style={{ width: charWidth, height: charHeight }}
      />
    </div>
  );
}
