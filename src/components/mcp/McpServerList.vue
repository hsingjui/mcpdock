<script setup lang="ts">
import { Plus, RefreshCw, Search, Upload } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { McpServer } from '../../types/mcp';
import McpServerCard from './McpServerCard.vue';
import type { ExpandKey, McpStatus } from './shared';

interface RuntimeCounts {
  tools: number;
  prompts: number;
  resources: number;
}

interface Props {
  servers: McpServer[];
  filteredServers: McpServer[];
  loading: boolean;
  refreshing: boolean;
  searchQuery: string;
  togglingIds: Set<number>;
  refreshingIds: Set<number>;
  statusOf: (server: McpServer) => McpStatus;
  transportLabel: (transport: McpServer['transportType']) => string;
  transportBadgeClass: (transport: McpServer['transportType']) => string;
  statusBadgeClass: (status: McpStatus) => string;
  statusDotClass: (status: McpStatus) => string;
  statusText: (status: McpStatus) => string;
  runtimeError: (serverId: number) => string | undefined;
  runtimeCounts: (serverId: number) => RuntimeCounts;
  getToolPayloads: (serverId: number) => unknown[];
  getPromptPayloads: (serverId: number) => unknown[];
  getResourcePayloads: (serverId: number) => unknown[];
  getResourceTemplatePayloads: (serverId: number) => unknown[];
  isExpanded: (serverId: number, key: ExpandKey) => boolean;
  toggleExpand: (serverId: number, key: ExpandKey) => void;
  isItemDisabled: (serverId: number, type: ExpandKey, index: number) => boolean;
  toggleItemDisabled: (serverId: number, type: ExpandKey, index: number) => void;
  isItemExpanded: (serverId: number, type: ExpandKey, index: number) => boolean;
  toggleItemExpanded: (serverId: number, type: ExpandKey, index: number) => void;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  'update:searchQuery': [value: string];
  create: [];
  import: [];
  refreshAll: [];
  edit: [server: McpServer];
  refresh: [serverId: number];
  toggle: [server: McpServer];
  delete: [serverId: number];
  runTool: [serverId: number, tool: unknown];
}>();

const { t } = useI18n();
</script>

<template>
  <div class="mt-4">
    <div class="mb-3 flex items-center justify-between gap-3">
      <div class="flex items-center gap-2">
        <n-tag size="small" :bordered="false" type="info">
          {{ t('mcp.serverCount', { count: servers.length }) }}
        </n-tag>
        <n-tag v-if="searchQuery.trim()" size="small" :bordered="false" type="warning">
          {{ t('mcp.matchCount', { count: filteredServers.length }) }}
        </n-tag>
      </div>
      <div class="flex items-center gap-2">
        <div class="relative">
          <n-input
            :value="searchQuery"
            :placeholder="t('mcp.searchPlaceholder')"
            size="medium"
            clearable
            class="w-48"
            @update:value="(value: string) => emit('update:searchQuery', value)"
          >
            <template #prefix>
              <Search class="h-3.5 w-3.5" />
            </template>
          </n-input>
        </div>
        <n-button size="medium" @click="emit('import')">
          <template #icon>
            <Upload class="h-3.5 w-3.5" />
          </template>
          {{ t('common.import') }}
        </n-button>
        <n-button size="medium" :loading="props.refreshing" @click="emit('refreshAll')">
          <template #icon>
            <RefreshCw class="h-3.5 w-3.5" />
          </template>
          {{ t('common.refresh') }}
        </n-button>
        <n-button type="primary" size="medium" @click="emit('create')">
          <template #icon>
            <Plus class="h-3.5 w-3.5" />
          </template>
          {{ t('mcp.addInstance') }}
        </n-button>
      </div>
    </div>

    <div class="overflow-hidden rounded-lg border border-light bg-base shadow-soft">
      <n-empty v-if="loading && servers.length === 0" :description="t('mcp.emptyDescription')" class="py-8" />
      <n-empty
        v-else-if="servers.length === 0"
        :description="t('mcp.emptyDescription')"
        class="py-8"
      />
      <n-empty
        v-else-if="filteredServers.length === 0"
        :description="t('mcp.emptyMatch')"
        class="py-8"
      />
      <McpServerCard
        v-for="(server, idx) in filteredServers"
        :key="server.id"
        :server="server"
        :index="idx"
        :total="filteredServers.length"
        :status="statusOf(server)"
        :runtime-error="runtimeError(server.id)"
        :runtime-counts="runtimeCounts(server.id)"
        :transport-label="transportLabel(server.transportType)"
        :transport-badge-class="transportBadgeClass(server.transportType)"
        :status-badge-class="statusBadgeClass(statusOf(server))"
        :status-dot-class="statusDotClass(statusOf(server))"
        :status-text="statusText(statusOf(server))"
        :tool-payloads="getToolPayloads(server.id)"
        :prompt-payloads="getPromptPayloads(server.id)"
        :resource-payloads="getResourcePayloads(server.id)"
        :resource-template-payloads="getResourceTemplatePayloads(server.id)"
        :refreshing="refreshingIds.has(server.id)"
        :toggling="togglingIds.has(server.id)"
        :is-expanded="isExpanded"
        :toggle-expand="toggleExpand"
        :is-item-disabled="isItemDisabled"
        :toggle-item-disabled="toggleItemDisabled"
        :is-item-expanded="isItemExpanded"
        :toggle-item-expanded="toggleItemExpanded"
        @edit="emit('edit', $event)"
        @refresh="emit('refresh', $event)"
        @toggle="emit('toggle', $event)"
        @delete="emit('delete', $event)"
        @run-tool="(serverId, tool) => emit('runTool', serverId, tool)"
      />
    </div>
  </div>
</template>
