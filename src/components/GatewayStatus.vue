<script setup lang="ts">
import { AlertTriangle, Circle, RotateCw } from '@lucide/vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useMessage } from 'naive-ui';
import { onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';

interface GatewayStatus {
  running: boolean;
  port: number | null;
  error: string | null;
}

const { t } = useI18n();
const status = ref<GatewayStatus>({ running: false, port: null, error: null });
const restarting = ref(false);
const message = useMessage();

async function fetchStatus(): Promise<void> {
  try {
    status.value = await invoke<GatewayStatus>('get_gateway_status');
  } catch {
    status.value = { running: false, port: null, error: t('gateway.queryFailed') };
  }
}

async function handleRestart(): Promise<void> {
  if (restarting.value) return;
  restarting.value = true;
  try {
    const result = await invoke<GatewayStatus>('restart_gateway');
    if (result.running) {
      message.success(t('gateway.restartSuccess', { port: result.port }));
    }
  } catch (err) {
    message.error(t('gateway.restartFailed', { error: err }));
  } finally {
    restarting.value = false;
  }
}

let unlisten: (() => void) | null = null;

onMounted(() => {
  void (async () => {
    unlisten = await listen<GatewayStatus>('gateway:status-changed', (event) => {
      status.value = event.payload;
      restarting.value = false;
    });
  })();

  requestAnimationFrame(() => {
    void fetchStatus();
  });
});

onUnmounted(() => {
  if (unlisten) {
    unlisten();
  }
});
</script>

<template>
  <div class="flex w-full items-center gap-2 text-[13px] font-medium">
    <!-- Running state -->
    <template v-if="status.running">
      <span class="relative flex h-2.5 w-2.5 shrink-0">
        <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-60" />
        <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-green-500" />
      </span>
      <span class="text-sub">{{ t('gateway.running') }}</span>
      <span class="rounded bg-tag-emerald-bg px-1.5 py-0.5 font-mono text-[11px] font-semibold text-tag-emerald-text">:{{ status.port }}</span>
      <n-button quaternary circle size="small" class="ml-auto" :disabled="restarting" @click="handleRestart">
        <template #icon>
          <RotateCw class="h-3.5 w-3.5" :class="{ 'animate-spin': restarting }" />
        </template>
      </n-button>
    </template>

    <!-- Error state -->
    <template v-else-if="status.error">
      <NTooltip placement="top" :style="{ maxWidth: '320px' }">
        <template #trigger>
          <div class="flex w-full items-center gap-2">
            <AlertTriangle class="h-4 w-4 shrink-0 text-warning" />
            <span class="text-tag-amber-text">{{ t('gateway.error') }}</span>
            <n-button
              quaternary
              circle
              size="small"
              type="warning"
              class="ml-auto"
              :disabled="restarting"
              @click="handleRestart"
            >
              <template #icon>
                <RotateCw class="h-3.5 w-3.5" :class="{ 'animate-spin': restarting }" />
              </template>
            </n-button>
          </div>
        </template>
        <span>{{ status.error }}</span>
      </NTooltip>
    </template>

    <!-- Stopped state -->
    <template v-else>
      <Circle class="h-2.5 w-2.5 shrink-0 text-placeholder" />
      <span class="text-sub">{{ t('gateway.stopped') }}</span>
      <n-button quaternary circle size="small" class="ml-auto" :disabled="restarting" @click="handleRestart">
        <template #icon>
          <RotateCw class="h-3.5 w-3.5" :class="{ 'animate-spin': restarting }" />
        </template>
      </n-button>
    </template>
  </div>
</template>