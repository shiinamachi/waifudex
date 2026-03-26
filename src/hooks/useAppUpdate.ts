import { useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";

const GET_APP_UPDATE_STATE_COMMAND = "get_app_update_state";
const CHECK_FOR_UPDATES_COMMAND = "check_for_updates_command";
const RESTART_TO_APPLY_UPDATE_COMMAND = "restart_to_apply_update_command";
const POLLING_STATUSES = new Set<AppUpdateStatus>([
  "checking",
  "downloading",
  "installing",
]);
const POLL_INTERVAL_MS = 1_000;

export type AppUpdateStatus =
  | "disabled"
  | "idle"
  | "checking"
  | "downloading"
  | "installing"
  | "ready_to_restart"
  | "up_to_date"
  | "error";

export interface AppUpdateSnapshot {
  currentVersion: string;
  availableVersion: string | null;
  status: AppUpdateStatus;
  lastCheckedAt: string | null;
  lastError: string | null;
  shouldPromptRestart: boolean;
}

interface AppUpdateStoreSnapshot {
  isLoaded: boolean;
  update: AppUpdateSnapshot;
}

const DEFAULT_UPDATE_SNAPSHOT: AppUpdateSnapshot = {
  currentVersion: "Unknown",
  availableVersion: null,
  status: "idle",
  lastCheckedAt: null,
  lastError: null,
  shouldPromptRestart: false,
};

let snapshot: AppUpdateStoreSnapshot = {
  isLoaded: false,
  update: DEFAULT_UPDATE_SNAPSHOT,
};

let initializePromise: Promise<void> | null = null;
let pollingTimer: ReturnType<typeof setTimeout> | null = null;
const subscribers = new Set<() => void>();

function emitStoreChange() {
  for (const subscriber of subscribers) {
    subscriber();
  }
}

function getStatusText(update: AppUpdateSnapshot): string {
  switch (update.status) {
    case "disabled":
      return "Updates are disabled in development builds.";
    case "idle":
      return "Ready to check for updates.";
    case "checking":
      return "Checking for updates...";
    case "downloading":
      return update.availableVersion
        ? `Downloading ${update.availableVersion}...`
        : "Downloading update...";
    case "installing":
      return update.availableVersion
        ? `Installing ${update.availableVersion}...`
        : "Installing update...";
    case "ready_to_restart":
      return update.availableVersion
        ? `${update.availableVersion} is ready. Restart to apply it.`
        : "An update is ready. Restart to apply it.";
    case "up_to_date":
      return "You are up to date.";
    case "error":
      return update.lastError ?? "Update failed.";
  }
}

function clearPolling() {
  if (pollingTimer !== null) {
    clearTimeout(pollingTimer);
    pollingTimer = null;
  }
}

function syncPolling(status: AppUpdateStatus) {
  if (subscribers.size === 0) {
    clearPolling();
    return;
  }

  if (POLLING_STATUSES.has(status)) {
    clearPolling();
    pollingTimer = setTimeout(() => {
      pollingTimer = null;
      void refreshSnapshot();
    }, POLL_INTERVAL_MS);
    return;
  }

  clearPolling();
}

function updateSnapshot(nextUpdate: AppUpdateSnapshot, isLoaded: boolean) {
  snapshot = {
    isLoaded,
    update: nextUpdate,
  };
  emitStoreChange();
  syncPolling(nextUpdate.status);
}

async function refreshSnapshot() {
  try {
    const nextUpdate = await invoke<AppUpdateSnapshot>(GET_APP_UPDATE_STATE_COMMAND);
    updateSnapshot(nextUpdate, true);
  } catch (error) {
    console.error("failed to refresh app update state", error);
    syncPolling(snapshot.update.status);
  }
}

async function initializeStore() {
  if (initializePromise) {
    return initializePromise;
  }

  initializePromise = (async () => {
    try {
      const update = await invoke<AppUpdateSnapshot>(GET_APP_UPDATE_STATE_COMMAND);
      updateSnapshot(update, true);
    } catch (error) {
      console.error("failed to initialize app update store", error);
      updateSnapshot(snapshot.update, true);
    }
  })();

  return initializePromise;
}

function subscribe(subscriber: () => void) {
  subscribers.add(subscriber);
  void initializeStore();

  return () => {
    subscribers.delete(subscriber);
    if (subscribers.size === 0) {
      clearPolling();
      initializePromise = null;
    }
  };
}

function getSnapshot() {
  return snapshot;
}

export function useAppUpdate() {
  const { isLoaded, update } = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  async function checkForUpdates() {
    const nextUpdate = await invoke<AppUpdateSnapshot>(CHECK_FOR_UPDATES_COMMAND);
    updateSnapshot(nextUpdate, true);
    return nextUpdate;
  }

  async function restartToApply() {
    await invoke<void>(RESTART_TO_APPLY_UPDATE_COMMAND);
  }

  return {
    isLoaded,
    currentVersion: update.currentVersion,
    availableVersion: update.availableVersion,
    status: update.status,
    statusText: getStatusText(update),
    lastCheckedAt: update.lastCheckedAt,
    isChecking: POLLING_STATUSES.has(update.status),
    isReadyToRestart: update.status === "ready_to_restart",
    checkForUpdates,
    restartToApply,
  };
}
