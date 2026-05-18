<script setup lang="ts">
import {
  Clock,
  Download,
  Globe,
  HeartPulse,
  Network,
  Power,
  RotateCcw,
  Router,
  Save,
  Shield,
  Sun,
} from '@lucide/vue';
import { useMessage } from 'naive-ui';
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useSettingsStore } from '../stores/settings';
import { useUpdaterStore } from '../stores/updater';
import PageHeader from './PageHeader.vue';

const store = useSettingsStore();
const updaterStore = useUpdaterStore();
const message = useMessage();
const { t } = useI18n();
const initialized = ref(false);

const updateStatusText = computed(() => {
  if (updaterStore.checking) return t('settings.updateChecking');
  if (updaterStore.isDownloading) return t('settings.updateInstalling');
  if (updaterStore.isDownloaded || updaterStore.isInstalling || updaterStore.isInstalled) {
    return t('settings.updateDownloaded');
  }
  if (updaterStore.hasUpdate) {
    return t('settings.updateAvailableVersion', {
      version: updaterStore.availableVersion,
    });
  }
  if (updaterStore.checkedOnce) return t('settings.updateUpToDate');
  return t('settings.updateNotChecked');
});

onMounted(async () => {
  await Promise.all([store.fetchSettings(), updaterStore.refreshCurrentVersion()]);
  initialized.value = true;
});

async function handleSave(): Promise<void> {
  if (store.saving) return;
  try {
    await store.saveSettings();
    message.success(t('common.saveSuccess'));
  } catch {
    message.error(t('common.saveFailed', { error: store.error ?? t('common.unknownError') }));
  }
}

function handleReset(): void {
  store.resetForm();
  message.info(t('common.resetSuccess'));
}

async function handleCheckUpdate(): Promise<void> {
  const hasUpdate = await updaterStore.checkForUpdates();

  if (updaterStore.error) {
    message.error(t('settings.updateCheckFailed', { error: updaterStore.error }));
    return;
  }

  if (hasUpdate) {
    message.success(t('settings.updateFound', { version: updaterStore.availableVersion }));
    return;
  }

  message.info(t('settings.updateUpToDate'));
}

async function handleDownloadUpdate(): Promise<void> {
  if (
    !updaterStore.hasUpdate ||
    updaterStore.isDownloading ||
    updaterStore.isInstalling ||
    updaterStore.isInstalled
  ) {
    return;
  }

  try {
    await updaterStore.downloadUpdate();
  } catch {
    message.error(
      t('settings.updateInstallFailed', {
        error: updaterStore.error ?? t('common.unknownError'),
      }),
    );
  }
}

async function handleRestartUpdate(): Promise<void> {
  if (!updaterStore.isDownloaded || updaterStore.isInstalling || updaterStore.isInstalled) return;

  try {
    await updaterStore.installAndRelaunch();
  } catch {
    message.error(
      t('settings.updateInstallFailed', {
        error: updaterStore.error ?? t('common.unknownError'),
      }),
    );
  }
}
</script>

<template>
  <main class="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-base">
    <PageHeader :title="t('settings.title')" :description="t('settings.description')" />

    <div v-if="!initialized" class="flex flex-1 items-center justify-center">
      <n-spin size="large" />
    </div>
    <template v-else>
      <div class="min-h-0 flex-1 overflow-y-auto px-6 md:px-8">
        <div class="mx-auto mt-4 max-w-3xl pb-24">
        <div class="grid grid-cols-1 items-start gap-4 lg:grid-cols-2">
          <div class="flex flex-col gap-4">
            <!-- ─── Language ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-primary-light text-primary"><Globe :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.languageTitle') }}</h3>
                <p class="card-desc">{{ t('settings.languageDescription') }}</p>
              </div>
            </div>
            <div class="card-body space-y-3">
              <n-radio-group v-model:value="store.settings.locale">
                <n-radio-button value="zh-CN">{{ t('settings.zhCN') }}</n-radio-button>
                <n-radio-button value="en">{{ t('settings.en') }}</n-radio-button>
              </n-radio-group>
            </div>
          </div>

          <!-- ─── Theme ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-primary-light text-primary"><Sun :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.themeTitle') }}</h3>
                <p class="card-desc">{{ t('settings.themeDescription') }}</p>
              </div>
            </div>
            <div class="card-body space-y-3">
              <n-radio-group v-model:value="store.settings.theme">
                <n-radio-button value="light">{{ t('settings.themeLight') }}</n-radio-button>
                <n-radio-button value="dark">{{ t('settings.themeDark') }}</n-radio-button>
                <n-radio-button value="system">{{ t('settings.themeSystem') }}</n-radio-button>
              </n-radio-group>
            </div>
          </div>

          <!-- ─── Gateway Port ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-emerald-soft text-emerald"><Router :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.portTitle') }}</h3>
                <p class="card-desc">{{ t('settings.portDescription') }}</p>
              </div>
            </div>
            <div class="card-body space-y-3">
              <div>
                <label class="label">{{ t('settings.listenPort') }}</label>
                <n-input-number v-model:value="store.settings.port" :min="1" :max="65535" :step="1" class="w-full" placeholder="3000" size="small" :status="store.settings.port == null ? 'error' : undefined" />
                <p class="hint">{{ t('settings.portHint') }}</p>
                <p v-if="store.settings.port == null" class="hint !text-error">{{ t('settings.valueRequired') }}</p>
              </div>
              <div>
                <label class="label">{{ t('settings.separator') }}</label>
                <n-input v-model:value="store.settings.gatewaySeparator" placeholder="__" size="small" />
                <p class="hint">{{ t('settings.separatorHint') }}</p>
              </div>
            </div>
          </div>

          <!-- ─── Proxy ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-warning-soft text-warning"><Network :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.proxyTitle') }}</h3>
                <p class="card-desc">{{ t('settings.proxyDescription') }}</p>
              </div>
              <span v-if="store.settings.proxyUrl" class="rounded-full border border-tag-emerald-border bg-tag-emerald-bg px-2 py-0.5 text-[10px] font-medium text-tag-emerald-text">{{ t('settings.proxyConfigured') }}</span>
            </div>
            <div class="card-body space-y-3">
              <div>
                <label class="label">{{ t('settings.proxyUrl') }}</label>
                <n-input v-model:value="store.settings.proxyUrl" placeholder="http://127.0.0.1:7890" clearable size="small" />
                <p class="hint">{{ t('settings.proxyUrlHint') }}</p>
              </div>
            </div>
          </div>

          </div>

          <div class="flex flex-col gap-4">
            <!-- ─── Update ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-primary-light text-primary"><Download :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.updateTitle') }}</h3>
                <p class="card-desc">{{ t('settings.updateDescription') }}</p>
              </div>
            </div>
            <div class="card-body space-y-3">
              <div>
                <label class="label">{{ t('settings.currentVersion') }}</label>
                <p class="hint !mt-0">{{ updaterStore.currentVersion || '—' }}</p>
              </div>
              <div>
                <label class="label">{{ t('settings.updateStatus') }}</label>
                <p class="hint !mt-0">{{ updateStatusText }}</p>
              </div>
              <div v-if="updaterStore.isDownloading" class="flex flex-col gap-2">
                <div class="flex items-center justify-between text-xs text-sub">
                  <span>{{ t('updater.downloading') }}</span>
                  <span v-if="updaterStore.progressPercent !== null">{{ updaterStore.progressPercent }}%</span>
                </div>
                <n-progress
                  type="line"
                  :percentage="updaterStore.progressPercent ?? 0"
                  :show-indicator="false"
                  :height="8"
                  rail-color="var(--color-light)"
                  processing
                  border-radius="4px"
                  fill-border-radius="4px"
                />
                <p v-if="updaterStore.progressPercent !== null" class="hint !mt-0">
                  {{ t('settings.updateDownloadProgress', { percent: updaterStore.progressPercent }) }}
                </p>
              </div>
              <div
                v-if="updaterStore.isDownloaded || updaterStore.isInstalling || updaterStore.isInstalled"
                class="flex items-center gap-2 rounded-lg bg-success-soft px-3 py-2 text-xs text-tag-emerald-text"
              >
                {{ t('updater.downloadComplete') }}
              </div>
              <div class="flex gap-2">
                <n-button
                  size="small"
                  :loading="updaterStore.checking"
                  :disabled="updaterStore.checking || updaterStore.isDownloading || updaterStore.isInstalling || updaterStore.isInstalled"
                  @click="handleCheckUpdate"
                >
                  {{ t('settings.checkUpdate') }}
                </n-button>
                <n-button
                  v-if="!updaterStore.isDownloaded && !updaterStore.isInstalling && !updaterStore.isInstalled"
                  size="small"
                  type="primary"
                  :loading="updaterStore.isDownloading"
                  :disabled="!updaterStore.hasUpdate || updaterStore.checking || updaterStore.isDownloading || updaterStore.isInstalling || updaterStore.isInstalled"
                  @click="handleDownloadUpdate"
                >
                  {{ t('settings.installUpdate') }}
                </n-button>
                <n-button
                  v-else
                  size="small"
                  type="primary"
                  :loading="updaterStore.isInstalling"
                  :disabled="updaterStore.isInstalling || updaterStore.isInstalled"
                  @click="handleRestartUpdate"
                >
                  {{ t('settings.restartUpdate') }}
                </n-button>
              </div>
            </div>
          </div>

          <!-- ─── Auto-Start ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-violet-soft text-violet"><Power :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.autoStartTitle') }}</h3>
                <p class="card-desc">{{ t('settings.autoStartDescription') }}</p>
              </div>
              <n-switch v-model:value="store.settings.autoStartEnabled" size="small" />
            </div>
            <div class="card-slide" :class="{ 'is-expanded': store.settings.autoStartEnabled }">
              <div class="card-slide-inner">
                <div class="card-body space-y-3">
                  <n-radio-group v-model:value="store.settings.autoStartHidden">
                    <n-radio-button :value="false">{{ t('settings.autoStartShowWindow') }}</n-radio-button>
                    <n-radio-button :value="true">{{ t('settings.autoStartHideWindow') }}</n-radio-button>
                  </n-radio-group>
                </div>
              </div>
            </div>
            <div class="card-slide" :class="{ 'is-expanded': !store.settings.autoStartEnabled }">
              <div class="card-slide-inner">
                <div class="card-body">
                  <p class="hint text-center" style="margin:0">{{ t('settings.notEnabled') }}</p>
                </div>
              </div>
            </div>
          </div>

            <!-- ─── Auth ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-info-soft text-info"><Shield :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.authTitle') }}</h3>
                <p class="card-desc">{{ t('settings.authDescription') }}</p>
              </div>
              <n-switch v-model:value="store.settings.authEnabled" size="small" />
            </div>
            <div class="card-slide" :class="{ 'is-expanded': store.settings.authEnabled }">
              <div class="card-slide-inner">
                <div class="card-body space-y-3">
                  <div>
                    <label class="label">{{ t('settings.authToken') }}</label>
                    <n-input v-model:value="store.settings.authToken" type="password" show-password-on="click" :placeholder="t('settings.authToken')" size="small" />
                  </div>
                </div>
              </div>
            </div>
            <div class="card-slide" :class="{ 'is-expanded': !store.settings.authEnabled }">
              <div class="card-slide-inner">
                <div class="card-body">
                  <p class="hint text-center" style="margin:0">{{ t('settings.notEnabled') }}</p>
                </div>
              </div>
            </div>
          </div>

          <!-- ─── Timeout ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-error-soft text-error"><Clock :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.timeoutTitle') }}</h3>
                <p class="card-desc">{{ t('settings.timeoutDescription') }}</p>
              </div>
              <n-switch v-model:value="store.settings.requestTimeoutEnabled" size="small" />
            </div>
            <div class="card-slide" :class="{ 'is-expanded': store.settings.requestTimeoutEnabled }">
              <div class="card-slide-inner">
                <div class="card-body space-y-3">
                  <div>
                    <label class="label">{{ t('settings.requestTimeoutMs') }}</label>
                    <n-input-number v-model:value="store.settings.requestTimeoutMs" :min="1000" :step="1000" class="w-full" placeholder="60000" size="small" :status="store.settings.requestTimeoutEnabled && store.settings.requestTimeoutMs == null ? 'error' : undefined" />
                    <p v-if="store.settings.requestTimeoutEnabled && store.settings.requestTimeoutMs == null" class="hint !text-error">{{ t('settings.valueRequired') }}</p>
                  </div>
                </div>
              </div>
            </div>
            <div class="card-slide" :class="{ 'is-expanded': !store.settings.requestTimeoutEnabled }">
              <div class="card-slide-inner">
                <div class="card-body">
                  <p class="hint text-center" style="margin:0">{{ t('settings.notEnabled') }}</p>
                </div>
              </div>
            </div>
          </div>

          <!-- ─── Keep-Alive ─── -->
          <div class="card">
            <div class="card-head">
              <div class="card-icon bg-success-soft text-emerald"><HeartPulse :size="15" /></div>
              <div class="min-w-0 flex-1">
                <h3 class="card-title">{{ t('settings.keepaliveTitle') }}</h3>
                <p class="card-desc">{{ t('settings.keepaliveDescription') }}</p>
              </div>
              <n-switch v-model:value="store.settings.keepAliveEnabled" size="small" />
            </div>
            <div class="card-slide" :class="{ 'is-expanded': store.settings.keepAliveEnabled }">
              <div class="card-slide-inner">
                <div class="card-body space-y-3">
                  <div>
                    <label class="label">{{ t('settings.keepaliveIntervalMs') }}</label>
                    <n-input-number v-model:value="store.settings.keepAliveIntervalMs" :min="5000" :step="5000" class="w-full" placeholder="60000" size="small" :status="store.settings.keepAliveEnabled && store.settings.keepAliveIntervalMs == null ? 'error' : undefined" />
                    <p v-if="store.settings.keepAliveEnabled && store.settings.keepAliveIntervalMs == null" class="hint !text-error">{{ t('settings.valueRequired') }}</p>
                    <p v-else class="hint">{{ t('settings.keepaliveIntervalHint') }}</p>
                  </div>
                </div>
              </div>
            </div>
            <div class="card-slide" :class="{ 'is-expanded': !store.settings.keepAliveEnabled }">
              <div class="card-slide-inner">
                <div class="card-body">
                  <p class="hint text-center" style="margin:0">{{ t('settings.notEnabled') }}</p>
                </div>
              </div>
            </div>
          </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ─── error toast ─── -->
      <div v-if="store.error" class="mx-6 mb-2 shrink-0 md:mx-8">
        <div class="rounded-lg border border-tag-red-border bg-error-soft px-3 py-2 text-xs text-error">
          {{ store.error }}
        </div>
      </div>
      <!-- ─── sticky footer ─── -->
      <div class="shrink-0 flex h-14 items-center border-t border-divider-soft bg-base/90 px-6 backdrop-blur-sm md:px-8">
        <div class="flex w-full items-center justify-between">
          <n-button quaternary size="small" @click="handleReset">
            <template #icon><RotateCcw :size="14" /></template>
            {{ t('settings.resetDefaults') }}
          </n-button>
          <n-button type="primary" :loading="store.saving" :disabled="store.saving || !store.canSave" @click="handleSave">
            <template #icon><Save :size="15" /></template>
            {{ t('settings.saveSettings') }}
          </n-button>
        </div>
      </div>
    </template>
  </main>
</template>

<style scoped>
/* ─── Card ─── */
.card {
  display: flex;
  flex-direction: column;
  border-radius: 0.75rem;
  border: 1px solid var(--color-light);
  background: var(--color-base);
  box-shadow: 0 2px 10px -4px rgba(0, 0, 0, 0.05);
  transition: box-shadow 0.15s;
  overflow: hidden;
}
.card:hover {
  box-shadow: 0 4px 16px -4px rgba(0, 0, 0, 0.08);
}

.card-head {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 0.875rem;
  border-bottom: 1px solid var(--color-divider-soft);
  background: var(--color-surface);
}

.card-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: 0.375rem;
  flex-shrink: 0;
}

.card-title {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-main);
  line-height: 1;
}

.card-desc {
  margin-top: 0.125rem;
  font-size: 0.6875rem;
  color: var(--color-sub);
  line-height: 1.3;
}

.card-body {
  padding: 0.75rem 0.875rem 1rem;
}

/* ─── Slide animation (grid-template-rows) ─── */
.card-slide {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.25s ease-out;
}
.card-slide.is-expanded {
  grid-template-rows: 1fr;
}
.card-slide-inner {
  overflow: hidden;
}

/* ─── Form ─── */
.label {
  display: block;
  margin-bottom: 0.25rem;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--color-sub);
}

.hint {
  margin-top: 0.25rem;
  font-size: 0.6875rem;
  color: var(--color-sub);
  opacity: 0.8;
}
</style>