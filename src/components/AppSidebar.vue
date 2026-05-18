<script setup lang="ts">
import { Folder, type LucideIcon, Server, Settings } from '@lucide/vue';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

import logoUrl from '../assets/icon.png';

import GatewayStatus from './GatewayStatus.vue';

type NavIcon = 'server' | 'folder' | 'settings';

interface NavItem {
  key: string;
  labelKey: string;
  icon: NavIcon;
}

const { t } = useI18n();

const isWindows = computed(() => navigator.userAgent.toLowerCase().includes('windows'));

const navIconMap: Record<NavIcon, LucideIcon> = {
  server: Server,
  folder: Folder,
  settings: Settings,
};

interface Props {
  activePage?: string;
}

defineProps<Props>();

const emit = defineEmits<{
  navigate: [key: string];
}>();

const navItems: NavItem[] = [
  { key: 'mcp', labelKey: 'sidebar.mcp', icon: 'server' },
  { key: 'group', labelKey: 'sidebar.group', icon: 'folder' },
  { key: 'settings', labelKey: 'sidebar.settings', icon: 'settings' },
];
</script>

<template>
  <aside class="hidden h-full min-h-0 w-52 shrink-0 flex-col border-r border-divider-soft bg-base md:flex">
    <div
      class="flex shrink-0 border-b border-divider-soft"
      :class="isWindows
        ? 'h-20 items-center justify-center'
        : 'h-24 items-end px-6 pb-4'"
      data-tauri-drag-region
    >
      <div
        class="flex items-center gap-3"
        :class="isWindows ? '-ml-3' : ''"
        data-tauri-drag-region
      >
        <img
          :src="logoUrl"
          class="shrink-0 rounded-lg object-cover"
          :class="isWindows ? 'h-12 w-12' : 'h-10 w-10'"
          alt="logo"
        />
        <h2
          class="font-semibold tracking-wide text-main"
          :class="isWindows ? 'text-xl' : 'text-lg'"
        >MCPDock</h2>
      </div>
    </div>

    <nav class="min-h-0 flex-1 space-y-1 overflow-y-auto px-3 py-4">
      <n-button
        v-for="item in navItems"
        :key="item.key"
        block
        quaternary
        class="!h-11 !justify-start !rounded-lg !px-3 !text-sm !font-semibold transition-colors"
        :class="
          activePage === item.key
            ? '!bg-primary-light !text-primary'
            : '!text-sub hover:!bg-surface hover:!text-main'
        "
        @click="emit('navigate', item.key)"
      >
        <template #icon>
          <component :is="navIconMap[item.icon]" class="h-5 w-5 shrink-0" />
        </template>
        <span>{{ t(item.labelKey) }}</span>
      </n-button>
    </nav>

    <div class="shrink-0 flex h-14 items-center border-t border-divider-soft px-3">
      <GatewayStatus />
    </div>
  </aside>
</template>