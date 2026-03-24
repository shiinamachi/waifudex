import { useState, useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface CharacterVisibilitySnapshot {
  isLoaded: boolean;
  visible: boolean;
}

const CHARACTER_VISIBILITY_CHANGED_EVENT =
  "waifudex://character-visibility-changed";
const GET_CHARACTER_VISIBILITY_COMMAND = "get_character_visibility";
const SET_CHARACTER_VISIBILITY_COMMAND = "set_character_visibility";

export const DEFAULT_CHARACTER_VISIBILITY_SNAPSHOT: CharacterVisibilitySnapshot = {
  isLoaded: false,
  visible: true,
};

let snapshot = DEFAULT_CHARACTER_VISIBILITY_SNAPSHOT;
let initializePromise: Promise<void> | null = null;
const subscribers = new Set<() => void>();

function emitStoreChange() {
  for (const subscriber of subscribers) {
    subscriber();
  }
}

function updateSnapshot(visible: boolean, isLoaded: boolean) {
  snapshot = {
    isLoaded,
    visible,
  };
  emitStoreChange();
}

async function initializeStore() {
  if (initializePromise) {
    return initializePromise;
  }

  initializePromise = (async () => {
    try {
      await listen<boolean>(CHARACTER_VISIBILITY_CHANGED_EVENT, ({ payload }) => {
        updateSnapshot(payload, true);
      });

      const visible = await invoke<boolean>(GET_CHARACTER_VISIBILITY_COMMAND);
      updateSnapshot(visible, true);
    } catch (error) {
      console.error("failed to initialize character visibility store", error);
      updateSnapshot(snapshot.visible, true);
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

export function useCharacterVisibility() {
  const { isLoaded, visible } = useSyncExternalStore(
    subscribe,
    getSnapshot,
    getSnapshot,
  );
  const [isSaving, setIsSaving] = useState(false);

  async function setVisible(nextVisible: boolean): Promise<boolean | undefined> {
    if (!isLoaded || isSaving) {
      return undefined;
    }

    setIsSaving(true);

    try {
      const resolvedVisible = await invoke<boolean>(SET_CHARACTER_VISIBILITY_COMMAND, {
        visible: nextVisible,
      });

      updateSnapshot(resolvedVisible, true);
      return resolvedVisible;
    } catch (error) {
      console.error("failed to update character visibility", error);
      throw error;
    } finally {
      setIsSaving(false);
    }
  }

  return {
    visible,
    isLoaded,
    isSaving,
    setVisible,
  };
}
