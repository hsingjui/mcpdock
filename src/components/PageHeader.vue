<script setup lang="ts">
import { computed } from 'vue';

import WindowControls from './WindowControls.vue';

interface Props {
  title: string;
  description?: string;
}

defineProps<Props>();

const isWindows = computed(() => navigator.userAgent.toLowerCase().includes('windows'));
</script>

<template>
  <header
    class="relative flex shrink-0 border-b border-divider-soft px-6 md:px-8"
    :class="isWindows
      ? 'h-20 items-center'
      : 'h-24 items-end pb-4'"
    data-tauri-drag-region
  >
    <div data-tauri-drag-region>
      <h1 class="text-xl font-semibold text-main">{{ title }}</h1>
      <p v-if="description" class="mt-1 text-sm text-sub">{{ description }}</p>
    </div>
    <div
      class="absolute top-2 right-3 flex items-center gap-2 md:right-4"
      data-tauri-no-drag-region
    >
      <slot />
      <WindowControls v-if="isWindows" />
    </div>
  </header>
</template>
