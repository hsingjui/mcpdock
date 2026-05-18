<script setup lang="ts">
import { listen } from '@tauri-apps/api/event';
import {
  createDiscreteApi,
  darkTheme,
  dateEnUS,
  dateZhCN,
  enUS,
  type GlobalThemeOverrides,
  zhCN,
} from 'naive-ui';
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import AppSidebar from './components/AppSidebar.vue';
import UpdateNotification from './components/UpdateNotification.vue';
import { i18n } from './i18n';
import { useMcpStore } from './stores/mcp';
import { isDarkRef } from './stores/settings';
import { useUpdaterStore } from './stores/updater';

const McpManagement = defineAsyncComponent(() => import('./components/McpManagement.vue'));
const GroupManagement = defineAsyncComponent(() => import('./components/GroupManagement.vue'));
const SettingsPage = defineAsyncComponent(() => import('./components/SettingsPage.vue'));

const STORAGE_KEY = 'mcpdock:activePage';
const activePage = ref(sessionStorage.getItem(STORAGE_KEY) ?? 'mcp');

watch(activePage, (val) => {
  sessionStorage.setItem(STORAGE_KEY, val);
});
const store = useMcpStore();
const { t } = useI18n();
const { message } = createDiscreteApi(['message']);

let unlistenGatewayError: (() => void) | undefined;

onUnmounted(() => {
  unlistenGatewayError?.();
});

const naiveLocaleMap: Record<string, typeof zhCN> = {
  'zh-CN': zhCN,
  en: enUS,
};
const naiveDateLocaleMap: Record<string, typeof dateZhCN> = {
  'zh-CN': dateZhCN,
  en: dateEnUS,
};

const naiveLocale = computed(() => {
  const locale = i18n.global.locale.value as string;
  return naiveLocaleMap[locale] ?? enUS;
});
const naiveDateLocale = computed(() => {
  const locale = i18n.global.locale.value as string;
  return naiveDateLocaleMap[locale] ?? dateEnUS;
});

const naiveTheme = computed(() => (isDarkRef.value ? darkTheme : undefined));

const lightThemeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#18a058',
    primaryColorHover: '#36ad6a',
    primaryColorPressed: '#0c7a43',
    primaryColorSuppl: '#36ad6a',
    bodyColor: '#ffffff',
    borderColor: '#e5e7eb',
    textColorBase: '#1f2937',
    textColor1: '#1f2937',
    textColor2: '#4b5563',
    borderRadius: '8px',
    borderRadiusSmall: '8px',
    fontFamily: 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
};

const darkThemeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#18a058',
    primaryColorHover: '#36ad6a',
    primaryColorPressed: '#0c7a43',
    primaryColorSuppl: '#36ad6a',
    bodyColor: '#101014',
    borderColor: '#3a3a3c',
    textColorBase: 'rgba(255, 255, 255, 0.82)',
    textColor1: 'rgba(255, 255, 255, 0.82)',
    textColor2: 'rgba(255, 255, 255, 0.52)',
    borderRadius: '8px',
    borderRadiusSmall: '8px',
    fontFamily: 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
};

const themeOverrides = computed(() => (isDarkRef.value ? darkThemeOverrides : lightThemeOverrides));

onMounted(async () => {
  // Start data loading immediately, don't block UI
  store.startListening();
  store.fetchServers();

  // Auto-check for updates only once per session (first launch or app restart)
  // Manual check from Settings always works regardless
  if (!sessionStorage.getItem('mcpdock:updateChecked')) {
    sessionStorage.setItem('mcpdock:updateChecked', '1');
    const updaterStore = useUpdaterStore();
    updaterStore.checkForUpdates().catch(() => {
      // Silently fail on startup — user can manually check in Settings
    });
  }

  unlistenGatewayError = await listen<string>('gateway:error', (event) => {
    message.error(t('gateway.startFailed', { error: event.payload }));
  });
});
</script>

<template>
  <n-config-provider :theme="naiveTheme" :theme-overrides="themeOverrides" :locale="naiveLocale" :date-locale="naiveDateLocale">
    <n-message-provider>
      <div class="flex h-screen min-h-0 w-screen overflow-hidden bg-base text-main">
        <AppSidebar :active-page="activePage" @navigate="activePage = $event" />

        <McpManagement v-if="activePage === 'mcp'" />
        <GroupManagement v-else-if="activePage === 'group'" />
        <SettingsPage v-else-if="activePage === 'settings'" />
      </div>

      <UpdateNotification />
    </n-message-provider>
  </n-config-provider>
</template>