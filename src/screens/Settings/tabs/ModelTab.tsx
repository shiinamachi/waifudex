import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Body1,
  Badge,
  Button,
  Caption1,
  Card,
  CardHeader,
  Spinner,
  tokens,
} from "@fluentui/react-components";

import {
  modelCard,
  modelCardActions,
  modelCardInfo,
  modelListHeader,
  tabsContainer,
} from "./tabs.css";

interface ModelEntry {
  fileName: string;
  displayName: string;
  path: string;
  isBundled: boolean;
  isActive: boolean;
}

const LIST_MODELS_COMMAND = "list_models";
const ADD_MODEL_COMMAND = "add_model";
const DELETE_MODEL_COMMAND = "delete_model";
const SWITCH_MODEL_COMMAND = "switch_model_command";

export default function ModelTab() {
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSwitching, setIsSwitching] = useState<string | null>(null);

  const loadModels = useCallback(async () => {
    try {
      const entries = await invoke<ModelEntry[]>(LIST_MODELS_COMMAND);
      setModels(entries);
    } catch (error) {
      console.error("failed to list models", error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadModels();
  }, [loadModels]);

  async function handleAddModel() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Inochi2D Model", extensions: ["inx"] }],
      });

      if (!selected) {
        return;
      }

      const entries = await invoke<ModelEntry[]>(ADD_MODEL_COMMAND, {
        sourcePath: selected,
      });
      setModels(entries);
    } catch (error) {
      console.error("failed to add model", error);
    }
  }

  async function handleDeleteModel(fileName: string) {
    try {
      const entries = await invoke<ModelEntry[]>(DELETE_MODEL_COMMAND, {
        fileName,
      });
      setModels(entries);
    } catch (error) {
      console.error("failed to delete model", error);
    }
  }

  async function handleSwitchModel(modelPath: string) {
    if (isSwitching) {
      return;
    }

    setIsSwitching(modelPath);
    try {
      const entries = await invoke<ModelEntry[]>(SWITCH_MODEL_COMMAND, {
        modelPath,
      });
      setModels(entries);
    } catch (error) {
      console.error("failed to switch model", error);
    } finally {
      setIsSwitching(null);
    }
  }

  if (isLoading) {
    return (
      <div className={tabsContainer}>
        <Spinner size="small" label="Loading models..." />
      </div>
    );
  }

  return (
    <div className={tabsContainer}>
      <div className={modelListHeader}>
        <Button appearance="primary" onClick={() => void handleAddModel()}>
          Add
        </Button>
      </div>

      {models.map((model) => (
        <Card key={model.fileName} appearance="filled">
          <CardHeader
            header={
              <div className={modelCard}>
                <div className={modelCardInfo}>
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: tokens.spacingHorizontalS,
                    }}
                  >
                    <Body1>{model.displayName}</Body1>
                    {model.isActive ? (
                      <Badge appearance="filled" color="brand" size="small">
                        Active
                      </Badge>
                    ) : null}
                    {model.isBundled ? (
                      <Badge appearance="outline" color="informative" size="small">
                        Built-in
                      </Badge>
                    ) : null}
                  </div>
                  <Caption1
                    style={{
                      color: tokens.colorNeutralForeground4,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {model.fileName}
                  </Caption1>
                </div>
                <div className={modelCardActions}>
                  {!model.isActive ? (
                    <Button
                      appearance="subtle"
                      size="small"
                      disabled={isSwitching !== null}
                      onClick={() => void handleSwitchModel(model.path)}
                    >
                      {isSwitching === model.path ? (
                        <Spinner size="tiny" />
                      ) : (
                        "Use"
                      )}
                    </Button>
                  ) : null}
                  {!model.isBundled && !model.isActive ? (
                    <Button
                      appearance="subtle"
                      size="small"
                      onClick={() => void handleDeleteModel(model.fileName)}
                    >
                      Delete
                    </Button>
                  ) : null}
                </div>
              </div>
            }
          />
        </Card>
      ))}

      {models.length === 0 ? (
        <Caption1 style={{ color: tokens.colorNeutralForeground4 }}>
          No models found.
        </Caption1>
      ) : null}
    </div>
  );
}
