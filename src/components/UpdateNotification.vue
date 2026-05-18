<script setup lang="ts">
import { CheckCircle2, Download, ExternalLink, X } from '@lucide/vue';
import { openUrl } from '@tauri-apps/plugin-opener';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useUpdaterStore } from '../stores/updater';

const updaterStore = useUpdaterStore();
const { t } = useI18n();

const showModal = ref(false);
const dismissed = ref(false);

const hasNewUpdate = computed(() => {
  const update = updaterStore.availableUpdate;
  return (
    updaterStore.hasUpdate &&
    update !== null &&
    !!update.version?.trim() &&
    !!update.currentVersion?.trim()
  );
});

const canClose = computed(
  () => !updaterStore.isDownloading && !updaterStore.isInstalling && !updaterStore.isInstalled,
);

const newVersion = computed(() => updaterStore.availableUpdate?.version ?? '');
const currentVersion = computed(() => updaterStore.availableUpdate?.currentVersion ?? '');
const releaseUrl = computed(
  () => `https://github.com/hsingjui/mcpdock/releases/tag/v${newVersion.value}`,
);

async function openRelease(): Promise<void> {
  await openUrl(releaseUrl.value);
}

const positiveText = computed(() => {
  if (updaterStore.installMode === 'portable') {
    return t('updater.openRelease');
  }
  if (updaterStore.isDownloaded || updaterStore.isInstalling || updaterStore.isInstalled) {
    return t('updater.restartNow');
  }
  return t('updater.downloadNow');
});

// Automatically show modal when an update is found
watch(
  hasNewUpdate,
  (val) => {
    if (val && !dismissed.value) {
      showModal.value = true;
    }
  },
  { immediate: true },
);

watch(
  () => updaterStore.checking,
  (checking) => {
    if (checking) {
      dismissed.value = false;
    }
  },
);

function handleDismiss(): void {
  if (!canClose.value) return;
  showModal.value = false;
  dismissed.value = true;
}

async function handlePositive(): Promise<void> {
  if (updaterStore.isDownloading || updaterStore.isInstalling || updaterStore.isInstalled) {
    return;
  }

  // Portable mode: just open the release page for manual download
  if (updaterStore.installMode === 'portable') {
    await openRelease();
    showModal.value = false;
    dismissed.value = true;
    return;
  }

  if (updaterStore.isDownloaded) {
    try {
      await updaterStore.installAndRelaunch();
    } catch {
      // Error is stored in updaterStore.error
    }
    return;
  }

  try {
    await updaterStore.downloadUpdate();
  } catch {
    // Error is stored in updaterStore.error
  }
}
</script>

<template>
  <n-modal
    v-model:show="showModal"
    :mask-closable="false"
    :close-on-esc="canClose"
    transform-origin="center"
  >
    <div class="mx-4" style="width: 34rem; max-width: calc(100vw - 2rem)">
      <n-card
        size="small"
        :bordered="false"
        role="dialog"
        aria-modal="true"
        class="w-full rounded-2xl shadow-soft"
      >
        <div class="flex flex-col">
          <!-- Header -->
          <div class="flex items-start justify-between gap-3">
            <div class="flex items-center gap-2.5">
              <div class="flex h-9 w-9 items-center justify-center rounded-xl bg-primary-light">
                <Download :size="18" class="text-primary" />
              </div>
              <h2 class="text-lg font-semibold text-main">
                {{ t('updater.updateAvailable') }}
              </h2>
            </div>
            <n-button quaternary circle size="small" :disabled="!canClose" @click="handleDismiss">
              <template #icon>
                <X :size="16" />
              </template>
            </n-button>
          </div>

          <!-- Version comparison -->
          <div class="mt-5 flex items-center justify-center gap-4">
            <div class="flex flex-col items-center rounded-xl bg-surface px-5 py-3">
              <span class="text-[11px] font-medium uppercase tracking-wide text-sub">
                {{ t('updater.current') }}
              </span>
              <span class="mt-0.5 text-base font-semibold text-sub">{{ currentVersion }}</span>
            </div>
            <div class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary-light">
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                <path d="M7 2v10M7 12l4-4M7 12l-4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="text-primary" />
              </svg>
            </div>
            <div class="flex flex-col items-center rounded-xl bg-primary-light px-5 py-3 ring-1 ring-primary/20">
              <span class="text-[11px] font-medium uppercase tracking-wide text-primary">
                {{ t('updater.latest') }}
              </span>
              <span class="mt-0.5 text-base font-bold text-primary">{{ newVersion }}</span>
            </div>
          </div>

          <!-- Progress -->
          <div v-if="updaterStore.isDownloading" class="mt-5 rounded-xl bg-surface px-4 py-3">
            <div class="mb-2 flex items-center justify-between text-xs">
              <span class="font-medium text-sub">{{ t('updater.downloading') }}</span>
              <span
                v-if="updaterStore.progressPercent !== null"
                class="tabular-nums font-semibold text-primary"
              >
                {{ updaterStore.progressPercent }}%
              </span>
            </div>
            <n-progress
              type="line"
              :percentage="updaterStore.progressPercent ?? 0"
              :show-indicator="false"
              :height="6"
              color="#18a058"
              rail-color="var(--color-light)"
              processing
              border-radius="999px"
              fill-border-radius="999px"
            />
          </div>

          <!-- Download complete -->
          <div
            v-if="updaterStore.isDownloaded || updaterStore.isInstalling || updaterStore.isInstalled"
            class="mt-5 flex items-center gap-2.5 rounded-xl bg-success-soft px-4 py-3"
          >
            <CheckCircle2 :size="18" class="shrink-0 text-tag-emerald-text" />
            <span class="text-sm font-medium text-tag-emerald-text">
              {{ t('updater.downloadComplete') }}
            </span>
          </div>

          <!-- Footer -->
          <div class="mt-5 flex items-center gap-2 border-t border-divider-soft pt-4">
            <n-button v-if="canClose" quaternary @click="handleDismiss">
              {{ t('updater.later') }}
            </n-button>
            <div class="ml-auto flex items-center gap-2">
              <n-button
                v-if="updaterStore.installMode !== 'portable'"
                secondary
                @click="openRelease"
              >
                <template #icon>
                  <ExternalLink :size="16" />
                </template>
                {{ $t('updater.openRelease') }}
              </n-button>
              <n-button
                type="primary"
                :loading="updaterStore.isDownloading || updaterStore.isInstalling"
                :disabled="updaterStore.isInstalled"
                @click="handlePositive"
              >
                {{ positiveText }}
              </n-button>
            </div>
          </div>
        </div>
      </n-card>
    </div>
  </n-modal>
</template>
