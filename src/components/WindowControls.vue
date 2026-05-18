<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, ref } from 'vue';

const appWindow = getCurrentWindow();
const isMaximized = ref(false);

async function syncMaximized(): Promise<void> {
  isMaximized.value = await appWindow.isMaximized();
}

void syncMaximized();

async function handleMinimize(): Promise<void> {
  await appWindow.minimize();
}

async function handleToggleMaximize(): Promise<void> {
  await appWindow.toggleMaximize();
  await syncMaximized();
}

async function handleClose(): Promise<void> {
  await appWindow.close();
}

const maximizeTitle = computed(() => (isMaximized.value ? '还原' : '最大化'));
</script>

<template>
  <div class="flex items-center gap-1.5" data-tauri-no-drag-region>
    <button
      type="button"
      class="flex h-10 w-10 items-center justify-center rounded-md text-sub transition-colors hover:bg-hover-bg hover:text-main"
      title="最小化"
      aria-label="最小化"
      @click="handleMinimize"
    >
      <svg viewBox="0 0 24 24" class="h-4.5 w-4.5" fill="none" stroke="currentColor" stroke-width="1.8">
        <path d="M5 12h14" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>

    <button
      type="button"
      class="flex h-10 w-10 items-center justify-center rounded-md text-sub transition-colors hover:bg-hover-bg hover:text-main"
      :title="maximizeTitle"
      :aria-label="maximizeTitle"
      @click="handleToggleMaximize"
    >
      <svg
        v-if="!isMaximized"
        viewBox="0 0 24 24"
        class="h-4.5 w-4.5"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
      >
        <rect x="5" y="5" width="14" height="14" rx="1.5" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      <svg
        v-else
        viewBox="0 0 24 24"
        class="h-4.5 w-4.5"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
      >
        <path d="M8 8h11v11H8z" stroke-linecap="round" stroke-linejoin="round" />
        <path d="M5 16V5h11" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>

    <button
      type="button"
      class="flex h-10 w-10 items-center justify-center rounded-md text-sub transition-colors hover:bg-error-soft hover:text-error"
      title="关闭"
      aria-label="关闭"
      @click="handleClose"
    >
      <svg viewBox="0 0 24 24" class="h-4.5 w-4.5" fill="none" stroke="currentColor" stroke-width="1.8">
        <path d="m7 7 10 10" stroke-linecap="round" stroke-linejoin="round" />
        <path d="M17 7 7 17" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
  </div>
</template>