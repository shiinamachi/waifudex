import { useState, useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const APP_SETTINGS_CHANGED_EVENT = "waifudex://app-settings-changed";
const GET_APP_SETTINGS_COMMAND = "get_app_settings";
const UPDATE_APP_SETTINGS_COMMAND = "update_app_settings_command";

export interface CharacterWindowPosition {
  x: number;
  y: number;
}

export interface AppSettings {
  alwaysOnTop: boolean;
  characterScale: number;
  displayMonitorId: string | null;
  characterWindowPosition: CharacterWindowPosition | null;
  activeModelPath: string | null;
}

export interface AppSettingsUpdate {
  alwaysOnTop?: boolean;
  characterScale?: number;
  displayMonitorId?: string | null;
  characterWindowPosition?: CharacterWindowPosition | null;
  activeModelPath?: string | null;
}

interface AppSettingsStoreSnapshot {
  isLoaded: boolean;
  settings: AppSettings;
}

const DEFAULT_APP_SETTINGS: AppSettings = {
  alwaysOnTop: true,
  characterScale: 1.0,
  displayMonitorId: null,
  characterWindowPosition: null,
  activeModelPath: null,
};

let snapshot: AppSettingsStoreSnapshot = {
  isLoaded: false,
  settings: DEFAULT_APP_SETTINGS,
};

let initializePromise: Promise<void> | null = null;
const subscribers = new Set<() => void>();

function emitStoreChange() {
  for (const subscriber of subscribers) {
    subscriber();
  }
}

function updateSnapshot(nextSettings: AppSettings, isLoaded: boolean) {
  snapshot = {
    isLoaded,
    settings: nextSettings,
  };
  emitStoreChange();
}

async function initializeStore() {
  if (initializePromise) {
    return initializePromise;
  }

  initializePromise = (async () => {
    try {
      await listen<AppSettings>(APP_SETTINGS_CHANGED_EVENT, ({ payload }) => {
        updateSnapshot(payload, true);
      });

      const settings = await invoke<AppSettings>(GET_APP_SETTINGS_COMMAND);
      updateSnapshot(settings, true);
    } catch (error) {
      console.error("failed to initialize app settings store", error);
      updateSnapshot(snapshot.settings, true);
    }
  })();

  return initializePromise;
}

function subscribe(subscriber: () => void) {
  subscribers.add(subscriber);
  void initializeStore();

  return () => {
    subscribers.delete(subscriber);
  };
}

function getSnapshot() {
  return snapshot;
}

export function useAppSettings() {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export function useAppSetting<K extends keyof AppSettings>(key: K) {
  const { isLoaded, settings } = useAppSettings();
  const [isSaving, setIsSaving] = useState(false);

  async function setValue(value: AppSettings[K]): Promise<AppSettings[K] | undefined> {
    if (!isLoaded || isSaving) {
      return undefined;
    }

    setIsSaving(true);

    try {
      const nextSettings = await invoke<AppSettings>(UPDATE_APP_SETTINGS_COMMAND, {
        update: {
          [key]: value,
        } satisfies AppSettingsUpdate,
      });

      updateSnapshot(nextSettings, true);
      return nextSettings[key];
    } catch (error) {
      console.error(`failed to update app setting: ${String(key)}`, error);
      throw error;
    } finally {
      setIsSaving(false);
    }
  }

  return {
    value: settings[key],
    isLoaded,
    isSaving,
    setValue,
  };
}
