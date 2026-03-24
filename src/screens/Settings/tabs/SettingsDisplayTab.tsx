import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Caption1,
  Dropdown,
  Option,
  Slider,
  tokens,
} from "@fluentui/react-components";

import SettingItem from "../../../components/settings/SettingItem";
import MonitorPreview from "../../../components/settings/MonitorPreview";
import {
  type CharacterWindowPosition,
  useAppSetting,
} from "../../../hooks/useAppSetting";

const BASE_WIDTH = 420;
const BASE_HEIGHT = 720;
const MIN_SCALE = 0.5;
const MAX_SCALE = 1.5;
const SCALE_STEP = 0.1;
const GET_DISPLAY_MONITORS_COMMAND = "get_display_monitors";
const UPDATE_APP_SETTINGS_COMMAND = "update_app_settings_command";

interface DisplayMonitorOption {
  id: string;
  label: string;
  workAreaLeft: number;
  workAreaTop: number;
  workAreaWidth: number;
  workAreaHeight: number;
}

export default function SettingsDisplayTab() {
  const { value: alwaysOnTop, setValue: setAlwaysOnTop } =
    useAppSetting("alwaysOnTop");
  const {
    value: displayMonitorId,
    setValue: setDisplayMonitorId,
    isLoaded,
  } = useAppSetting("displayMonitorId");
  const { value: characterScale, setValue: setCharacterScale } =
    useAppSetting("characterScale");
  const { value: characterWindowPosition } = useAppSetting(
    "characterWindowPosition",
  );

  const [previewScale, setPreviewScale] = useState<number | null>(null);
  const [monitorOptions, setMonitorOptions] = useState<DisplayMonitorOption[]>(
    [],
  );
  const inFlightMoveRef = useRef(false);
  const queuedMoveRef = useRef<CharacterWindowPosition | null>(null);
  const lastQueuedMoveRef = useRef<string | null>(null);

  const displayScale = previewScale ?? characterScale;
  const selectedMonitor = monitorOptions.find(
    (monitor) => monitor.id === displayMonitorId,
  );
  const previewMonitor = selectedMonitor ?? null;
  const selectedMonitorValue = selectedMonitor?.label ?? displayMonitorId ?? "";
  const selectedMonitorLabel = selectedMonitor?.label ?? displayMonitorId ?? null;
  const displayWidth = Math.round(BASE_WIDTH * displayScale);
  const displayHeight = Math.round(BASE_HEIGHT * displayScale);

  useEffect(() => {
    let cancelled = false;

    void (async () => {
      try {
        const nextOptions = await invoke<DisplayMonitorOption[]>(
          GET_DISPLAY_MONITORS_COMMAND,
        );
        if (!cancelled) {
          setMonitorOptions(nextOptions);
        }
      } catch (error) {
        console.error("failed to load display monitors", error);
        if (!cancelled) {
          setMonitorOptions([]);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  function queueCharacterWindowMove(position: CharacterWindowPosition) {
    const key = `${position.x}:${position.y}`;
    if (lastQueuedMoveRef.current === key) {
      return;
    }

    lastQueuedMoveRef.current = key;
    queuedMoveRef.current = position;
    if (inFlightMoveRef.current) {
      return;
    }

    inFlightMoveRef.current = true;
    void (async () => {
      try {
        while (queuedMoveRef.current) {
          const nextPosition = queuedMoveRef.current;
          queuedMoveRef.current = null;
          await invoke(UPDATE_APP_SETTINGS_COMMAND, {
            update: {
              characterWindowPosition: nextPosition,
            },
          });
        }
      } catch (error) {
        console.error("failed to move character window", error);
        lastQueuedMoveRef.current = null;
      } finally {
        inFlightMoveRef.current = false;
        if (queuedMoveRef.current) {
          queueCharacterWindowMove(queuedMoveRef.current);
        }
      }
    })();
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: tokens.spacingVerticalS,
      }}
    >
      <SettingItem
        title="Always on top"
        description="Always display the waifudex character at the top"
        type="switch"
        value={alwaysOnTop}
        onChange={setAlwaysOnTop}
      />

      <SettingItem
        title="Display monitor"
        description="Choose which monitor shows the character window"
      >
        <div
          style={{
            padding: `${tokens.spacingVerticalS} ${tokens.spacingHorizontalM}`,
          }}
        >
          <Dropdown
            appearance="outline"
            disabled={!isLoaded || monitorOptions.length === 0}
            onOptionSelect={(_event, data) => {
              const nextMonitorId = data.optionValue;
              if (nextMonitorId) {
                void setDisplayMonitorId(nextMonitorId);
              }
            }}
            placeholder="Select a monitor"
            selectedOptions={displayMonitorId ? [displayMonitorId] : []}
            style={{
              width: "100%",
            }}
            value={selectedMonitorValue}
          >
            {monitorOptions.map((monitor) => (
              <Option key={monitor.id} text={monitor.label} value={monitor.id}>
                {monitor.label}
              </Option>
            ))}
          </Dropdown>
        </div>
      </SettingItem>

      <SettingItem
        title="Character size and position"
        description="Adjust the character window size and drag the preview to reposition it"
      >
        <div
          style={{
            padding: `${tokens.spacingVerticalS} ${tokens.spacingHorizontalM}`,
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: tokens.spacingHorizontalS,
            }}
          >
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
          <MonitorPreview
            monitor={
              previewMonitor
                ? {
                    label: selectedMonitorLabel ?? previewMonitor.label,
                    workAreaLeft: previewMonitor.workAreaLeft,
                    workAreaTop: previewMonitor.workAreaTop,
                    workAreaWidth: previewMonitor.workAreaWidth,
                    workAreaHeight: previewMonitor.workAreaHeight,
                  }
                : null
            }
            onMoveCharacterWindow={queueCharacterWindowMove}
            position={characterWindowPosition}
            scale={displayScale}
          />
        </div>
      </SettingItem>
    </div>
  );
}
