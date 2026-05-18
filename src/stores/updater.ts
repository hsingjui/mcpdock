import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { defineStore } from 'pinia';
import { computed, ref, shallowRef } from 'vue';

interface AvailableUpdate {
  body?: string;
  currentVersion: string;
  date?: string;
  version: string;
}

type DownloadPhase = 'idle' | 'downloading' | 'downloaded' | 'installing' | 'installed';
type InstallMode = 'installed' | 'portable';

export const useUpdaterStore = defineStore('updater', () => {
  const currentVersion = ref('');
  const availableUpdate = ref<AvailableUpdate | null>(null);
  const checking = ref(false);
  const error = ref<string | null>(null);
  const checkedOnce = ref(false);
  const downloadedBytes = ref(0);
  const contentLength = ref<number | null>(null);
  const pendingUpdate = shallowRef<Update | null>(null);
  const phase = ref<DownloadPhase>('idle');
  const installMode = ref<InstallMode>('installed');

  const hasUpdate = computed(() => availableUpdate.value !== null);
  const availableVersion = computed(() => availableUpdate.value?.version ?? '');
  const progressPercent = computed(() => {
    if (!contentLength.value || contentLength.value <= 0) {
      return null;
    }

    return Math.min(100, Math.round((downloadedBytes.value / contentLength.value) * 100));
  });
  const isDownloading = computed(() => phase.value === 'downloading');
  const isDownloaded = computed(() => phase.value === 'downloaded');
  const isInstalling = computed(() => phase.value === 'installing');
  const isInstalled = computed(() => phase.value === 'installed');

  async function clearPendingUpdate(): Promise<void> {
    const update = pendingUpdate.value;
    pendingUpdate.value = null;
    availableUpdate.value = null;
    downloadedBytes.value = 0;
    contentLength.value = null;
    phase.value = 'idle';

    if (update) {
      try {
        await update.close();
      } catch {
        // noop
      }
    }
  }

  async function refreshCurrentVersion(): Promise<void> {
    try {
      currentVersion.value = await getVersion();
      installMode.value = await invoke<InstallMode>('install_mode');
    } catch (reason) {
      error.value = String(reason);
    }
  }

  async function checkForUpdates(): Promise<boolean> {
    checking.value = true;
    error.value = null;

    try {
      await refreshCurrentVersion();
      const update = await check();
      checkedOnce.value = true;

      if (!update) {
        await clearPendingUpdate();
        return false;
      }

      // Validate that the update contains meaningful version info;
      // the updater plugin may return a non-null Update with empty
      // version/currentVersion in dev builds or misconfigured endpoints.
      if (!update.version?.trim() || !update.currentVersion?.trim()) {
        await clearPendingUpdate();
        return false;
      }

      await clearPendingUpdate();
      pendingUpdate.value = update;
      availableUpdate.value = {
        body: update.body,
        currentVersion: update.currentVersion,
        date: update.date,
        version: update.version,
      };
      return true;
    } catch (reason) {
      error.value = String(reason);
      await clearPendingUpdate();
      return false;
    } finally {
      checking.value = false;
    }
  }

  async function downloadUpdate(): Promise<void> {
    const update = pendingUpdate.value;
    if (!update) {
      error.value = 'No pending update';
      return;
    }

    phase.value = 'downloading';
    error.value = null;
    downloadedBytes.value = 0;
    contentLength.value = null;

    try {
      await update.download((event) => {
        switch (event.event) {
          case 'Started':
            contentLength.value = event.data.contentLength ?? null;
            downloadedBytes.value = 0;
            break;
          case 'Progress':
            downloadedBytes.value += event.data.chunkLength;
            break;
          case 'Finished':
            break;
        }
      });
      phase.value = 'downloaded';
    } catch (reason) {
      error.value = String(reason);
      phase.value = 'idle';
      downloadedBytes.value = 0;
      contentLength.value = null;
      throw reason;
    }
  }

  async function installAndRelaunch(): Promise<void> {
    // Portable mode cannot auto-install – user must manually replace files
    if (installMode.value === 'portable') {
      phase.value = 'installed';
      return;
    }

    const update = pendingUpdate.value;
    if (!update) {
      error.value = 'No pending update';
      return;
    }

    phase.value = 'installing';

    try {
      await update.install();
      phase.value = 'installed';
      await relaunch();
    } catch (reason) {
      error.value = String(reason);
      phase.value = 'downloaded';
      throw reason;
    }
  }

  return {
    currentVersion,
    availableUpdate,
    checking,
    error,
    checkedOnce,
    downloadedBytes,
    contentLength,
    phase,
    hasUpdate,
    availableVersion,
    progressPercent,
    isDownloading,
    isDownloaded,
    isInstalling,
    isInstalled,
    installMode,
    refreshCurrentVersion,
    checkForUpdates,
    downloadUpdate,
    installAndRelaunch,
  };
});
