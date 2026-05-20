<script setup lang="ts">
import { Copy, Save } from '@lucide/vue';
import { useMessage } from 'naive-ui';
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useGroupStore } from '../stores/group';
import { useMcpStore } from '../stores/mcp';
import { useSettingsStore } from '../stores/settings';
import type { McpCapability, McpGroup, McpGroupServerSelection } from '../types/group';
import type { McpServer } from '../types/mcp';
import GroupForm from './group/GroupForm.vue';
import GroupList from './group/GroupList.vue';
import PageHeader from './PageHeader.vue';

const groupStore = useGroupStore();
const mcpStore = useMcpStore();
const settingsStore = useSettingsStore();
const { t } = useI18n();
const message = useMessage();

const showForm = ref(false);
const expandedServers = ref<Record<number, boolean>>({});
const scrollContainer = ref<HTMLElement | null>(null);
const savedScrollTop = ref(0);

const groups = computed(() => groupStore.groups);
const servers = computed(() => mcpStore.servers);
const form = computed(() => groupStore.form);
const isEditing = computed(() => groupStore.editingId !== null);

function saveScrollPosition(): void {
  if (scrollContainer.value) {
    savedScrollTop.value = scrollContainer.value.scrollTop;
  }
}

function restoreScrollPosition(): void {
  nextTick(() => {
    if (scrollContainer.value) {
      scrollContainer.value.scrollTop = savedScrollTop.value;
    }
  });
}

function handleKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape' && showForm.value) {
    closeForm();
  }
}

onMounted(async () => {
  document.addEventListener('keydown', handleKeydown);
  await Promise.all([
    groupStore.fetchGroups(),
    groupStore.fetchCapabilities(),
    settingsStore.fetchSettings(),
  ]);
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown);
});

function openCreate(): void {
  groupStore.resetForm();
  saveScrollPosition();
  showForm.value = true;
}

function openEdit(id: string): void {
  const group = groups.value.find((item) => item.id === id);
  if (!group) return;
  groupStore.startEdit(group);
  saveScrollPosition();
  showForm.value = true;
}

function closeForm(): void {
  groupStore.resetForm();
  expandedServers.value = {};
  showForm.value = false;
  restoreScrollPosition();
}

async function handleSubmit(): Promise<void> {
  await groupStore.submitForm();
  expandedServers.value = {};
  showForm.value = false;
  restoreScrollPosition();
}

async function handleDelete(id: string): Promise<void> {
  await groupStore.deleteGroup(id);
}

function toggleExpanded(serverId: number): void {
  expandedServers.value[serverId] = !expandedServers.value[serverId];
}

function capabilitiesOf(serverId: number, type: 'tool' | 'prompt' | 'resource'): McpCapability[] {
  return (groupStore.capabilityMap[serverId] ?? []).filter((item) => item.type === type);
}

function capabilityLabel(item: McpCapability): string {
  return item.name?.trim() ? item.name : item.capabilityKey;
}

function capabilityHint(item: McpCapability): string {
  return item.description?.trim() ? item.description : item.capabilityKey;
}

function selectedCount(sel: McpGroupServerSelection, type: 'tool' | 'prompt' | 'resource'): number {
  const total = capabilitiesOf(sel.serverId, type).length;
  const category = type === 'tool' ? 'tools' : type === 'prompt' ? 'prompts' : 'resources';
  const values = sel[category];
  if (values === null) return total;
  return values.length;
}

function groupTotalCount(group: McpGroup, type: 'tool' | 'prompt' | 'resource'): number {
  return group.config.servers
    .filter((sel) => serverEnabled(sel.serverId))
    .reduce((sum, sel) => sum + selectedCount(sel, type), 0);
}

function serverEnabled(serverId: number): boolean {
  return mcpStore.servers.find((s) => s.id === serverId)?.enabled ?? false;
}

function endpointJson(url: string): string {
  const config: Record<string, unknown> = { url };
  if (settingsStore.settings.authEnabled && settingsStore.settings.authToken.trim()) {
    config.headers = {
      // biome-ignore lint/style/useNamingConvention: standard HTTP header name
      Authorization: `Bearer ${settingsStore.settings.authToken}`,
    };
  }
  return JSON.stringify({ mcpServers: { mcpdock: config } }, null, 2);
}

function globalEndpointUrl(): string {
  const port = settingsStore.settings.port || 3100;
  return `http://localhost:${port}/mcp`;
}

function globalEndpointJson(): string {
  return endpointJson(globalEndpointUrl());
}

function groupUrl(group: McpGroup): string {
  const port = settingsStore.settings.port || 3100;
  const encodedName = encodeURIComponent(group.name);
  return `http://localhost:${port}/mcp/${encodedName}`;
}

function groupJson(group: McpGroup): string {
  return endpointJson(groupUrl(group));
}

async function copyToClipboard(text: string, label: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    message.success(t('common.copySuccess', { label }));
  } catch {
    message.error(t('common.copyFailed'));
  }
}

const copyOptions = [
  { label: t('group.copyUrl'), key: 'url' },
  { label: t('group.copyJson'), key: 'json' },
];

function handleGlobalCopy(key: string): void {
  if (key === 'url') {
    void copyToClipboard(globalEndpointUrl(), 'URL');
  } else if (key === 'json') {
    void copyToClipboard(globalEndpointJson(), 'JSON');
  }
}

function handleCopy(group: McpGroup, key: string): void {
  if (key === 'url') {
    void copyToClipboard(groupUrl(group), 'URL');
  } else if (key === 'json') {
    void copyToClipboard(groupJson(group), 'JSON');
  }
}

function handleNameInput(val: string): void {
  groupStore.form.name = val.replace(/[^a-zA-Z0-9_-]/g, '');
}

function statusText(server: McpServer): string {
  if (!server.enabled) {
    return t('common.disabled');
  }
  if (groupStore.isServerFullySelected(server.id)) {
    return t('group.fullySelected');
  }
  return t('group.partiallySelected');
}
</script>

<template>
  <main class="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-base">
    <PageHeader :title="t('group.title')" :description="t('group.description')" />

    <div ref="scrollContainer" class="min-h-0 flex-1 overflow-y-auto px-6 pb-24 md:px-8">
      <div v-if="!showForm" class="mt-4 flex flex-col gap-4">
        <div class="flex flex-col gap-3 rounded-xl border border-primary/20 bg-info-soft p-5 shadow-soft">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <h2 class="text-base font-semibold text-main">{{ t('group.globalEndpoint') }}</h2>
              <p class="mt-1 text-sm text-sub">{{ t('group.globalEndpointDescription') }}</p>
            </div>
            <n-dropdown :options="copyOptions" trigger="click" @select="handleGlobalCopy">
              <n-button quaternary size="small" class="shrink-0">
                <template #icon>
                  <Copy class="h-3.5 w-3.5" />
                </template>
              </n-button>
            </n-dropdown>
          </div>
          <div class="flex items-center justify-between rounded-lg border border-light bg-base px-3 py-2">
            <div class="flex min-w-0 items-center gap-2">
              <span class="shrink-0 text-xs font-medium text-disabled">URL</span>
              <span class="truncate text-xs font-mono text-main select-all">{{ globalEndpointUrl() }}</span>
            </div>
          </div>
        </div>

        <GroupList
          :groups="groups"
          :loading="groupStore.loading"
          :error="groupStore.error"
          :copy-options="copyOptions"
          :group-url="groupUrl"
          :server-enabled="serverEnabled"
          :selected-count="selectedCount"
          :group-total-count="groupTotalCount"
          @create="openCreate"
          @edit="openEdit"
          @delete="handleDelete"
          @copy="handleCopy"
        />
      </div>

      <GroupForm
        v-else
        :servers="servers"
        :form-name="form.name"
        :is-editing="isEditing"
        :expanded-servers="expandedServers"
        :loading="groupStore.loading"
        :error="groupStore.error"
        :has-server="groupStore.hasServer"
        :toggle-server="groupStore.toggleServer"
        :capabilities-of="capabilitiesOf"
        :capability-label="capabilityLabel"
        :capability-hint="capabilityHint"
        :status-text="statusText"
        :is-capability-selected="groupStore.isCapabilitySelected"
        :toggle-capability="groupStore.toggleCapability"
        :set-category-all="groupStore.setCategoryAll"
        :set-category-none="groupStore.setCategoryNone"
        @close="closeForm"
        @submit="handleSubmit"
        @toggle-expanded="toggleExpanded"
        @update-name="handleNameInput"
      />
    </div>

    <div
      v-if="showForm"
      class="sticky bottom-0 flex h-14 items-center border-t border-divider-soft bg-base/90 px-6 backdrop-blur-sm md:px-8"
    >
      <div class="flex w-full items-center justify-between">
        <n-button quaternary size="small" @click="closeForm">{{ t('common.cancel') }}</n-button>
        <n-button type="primary" :loading="groupStore.saving" @click="handleSubmit">
          <template #icon><Save class="h-4 w-4" /></template>
          {{ t('common.save') }}
        </n-button>
      </div>
    </div>
  </main>
</template>
