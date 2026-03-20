import { useState } from "react";
import { Caption1, Slider, tokens } from "@fluentui/react-components";

import SettingItem from "../../../components/settings/SettingItem";
import MonitorPreview from "../../../components/settings/MonitorPreview";
import { useAppSetting } from "../../../hooks/useAppSetting";

const BASE_WIDTH = 420;
const BASE_HEIGHT = 720;
const MIN_SCALE = 0.5;
const MAX_SCALE = 1.5;
const SCALE_STEP = 0.1;

export default function SettingsDisplayTab() {
  const { value: alwaysOnTop, setValue: setAlwaysOnTop } =
    useAppSetting("alwaysOnTop");
  const { value: characterScale, setValue: setCharacterScale } =
    useAppSetting("characterScale");

  const [previewScale, setPreviewScale] = useState<number | null>(null);

  const displayScale = previewScale ?? characterScale;
  const displayWidth = Math.round(BASE_WIDTH * displayScale);
  const displayHeight = Math.round(BASE_HEIGHT * displayScale);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: tokens.spacingVerticalS }}>
      <SettingItem
        title="Always on top"
        description="Always display the waifudex character at the top"
        type="switch"
        value={alwaysOnTop}
        onChange={setAlwaysOnTop}
      />

      <SettingItem
        title="Character size"
        description="Adjust the size of the character window"
      >
        <div style={{ padding: `${tokens.spacingVerticalS} ${tokens.spacingHorizontalM}` }}>
          <div style={{ display: "flex", alignItems: "center", gap: tokens.spacingHorizontalS }}>
            <Slider
              min={MIN_SCALE}
              max={MAX_SCALE}
              step={SCALE_STEP}
              value={displayScale}
              onChange={(_event, data) => setPreviewScale(data.value)}
              onPointerUp={() => {
                if (previewScale !== null) {
                  setCharacterScale(previewScale);
                  setPreviewScale(null);
                }
              }}
              onKeyUp={() => {
                if (previewScale !== null) {
                  setCharacterScale(previewScale);
                  setPreviewScale(null);
                }
              }}
              style={{ flex: 1 }}
            />
            <Caption1 style={{ minWidth: "3em", textAlign: "right" }}>
              {displayScale.toFixed(1)}x
            </Caption1>
          </div>
          <Caption1 style={{ color: tokens.colorNeutralForeground4 }}>
            {displayWidth} x {displayHeight}
          </Caption1>
          <MonitorPreview scale={displayScale} />
        </div>
      </SettingItem>
    </div>
  );
}
