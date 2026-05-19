<script setup lang="ts">
import { Play, Save } from '@lucide/vue';
import { computed, nextTick, onMounted, onUnmounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { McpHubImportResult, McpImportResult } from '../stores/mcp';
import { useMcpStore } from '../stores/mcp';
import type { McpServer } from '../types/mcp';
import McpImportView from './mcp/McpImportView.vue';
import McpServerForm from './mcp/McpServerForm.vue';
import McpServerList from './mcp/McpServerList.vue';
import McpToolRunner from './mcp/McpToolRunner.vue';
import {
  type ExpandKey,
  extractToolParams,
  type KeyValueEntry,
  type McpStatus,
  type ParamField,
  transportLabel,
} from './mcp/shared';
import PageHeader from './PageHeader.vue';

const store = useMcpStore();
const { t } = useI18n();

const showFormModal = ref(false);
const showImportView = ref(false);
const showRunToolView = ref(false);

const importJson = ref('');
const importLoading = ref(false);
const importResult = ref<McpImportResult | null>(null);
const mcpHubImportResult = ref<McpHubImportResult | null>(null);

const envExpanded = ref(false);
const headersExpanded = ref(false);
const argsExpanded = ref(false);
const envEntries = ref<KeyValueEntry[]>([{ key: '', value: '' }]);
const headerEntries = ref<KeyValueEntry[]>([{ key: '', value: '' }]);
const argsEntries = ref<string[]>(['']);

const scrollContainer = ref<HTMLElement | null>(null);
const savedScrollTop = ref(0);
const searchQuery = ref('');

const runToolServerId = ref<number | null>(null);
const runToolName = ref('');
const runToolDescription = ref('');
const runToolParams = ref<ParamField[]>([]);
const runToolArgs = reactive<Record<string, unknown>>({});
const runToolLoading = ref(false);
const runToolResult = ref<string | null>(null);
const runToolError = ref<string | null>(null);

const servers = computed(() => store.servers);
const form = computed(() => store.form);
const isEditing = computed(() => store.editingId !== null);
const filteredServers = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return servers.value;
  return servers.value.filter((s) => s.name.toLowerCase().includes(q));
});

const capabilitySummaryMap = computed(() => {
  const summary = new Map<
    number,
    {
      tools: unknown[];
      prompts: unknown[];
      resources: unknown[];
      resourceTemplates: unknown[];
    }
  >();

  for (const capability of store.capabilities) {
    const current = summary.get(capability.serverId) ?? {
      tools: [],
      prompts: [],
      resources: [],
      resourceTemplates: [],
    };

    if (capability.type === 'tool') {
      current.tools.push(capability.payload);
    } else if (capability.type === 'prompt') {
      current.prompts.push(capability.payload);
    } else if (
      capability.payload &&
      typeof capability.payload === 'object' &&
      'uriTemplate' in (capability.payload as object)
    ) {
      current.resourceTemplates.push(capability.payload);
    } else if (capability.type === 'resource') {
      current.resources.push(capability.payload);
    }

    summary.set(capability.serverId, current);
  }

  return summary;
});

onMounted(() => {
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown);
});

function statusOf(server: McpServer): McpStatus {
  if (!server.enabled) return 'disabled';
  const runtime = store.runtimeFor(server.id);
  if (runtime.connecting) return 'connecting';
  if (runtime.error) return 'error';
  if (runtime.connected) return 'running';
  return 'stopped';
}

function statusDotClass(status: McpStatus): string {
  switch (status) {
    case 'running':
      return 'bg-emerald';
    case 'error':
      return 'bg-error';
    case 'connecting':
      return 'bg-warning';
    default:
      return 'bg-placeholder';
  }
}

function transportBadgeClass(transport: McpServer['transportType']): string {
  switch (transport) {
    case 'stdio':
      return 'bg-tag-slate-bg text-tag-slate-text';
    case 'streamable_http':
      return 'bg-tag-emerald-bg text-tag-emerald-text';
  }
}

function statusBadgeClass(status: McpStatus): string {
  switch (status) {
    case 'running':
      return 'border-tag-emerald-border bg-tag-emerald-bg text-tag-emerald-text';
    case 'error':
      return 'border-tag-red-border bg-tag-red-bg text-tag-red-text';
    case 'connecting':
      return 'border-tag-amber-border bg-tag-amber-bg text-tag-amber-text';
    default:
      return 'border-tag-gray-border bg-tag-gray-bg text-tag-gray-text';
  }
}

function statusText(status: McpStatus): string {
  switch (status) {
    case 'running':
      return t('mcp.statusRunning');
    case 'error':
      return t('mcp.statusError');
    case 'connecting':
      return t('mcp.statusConnecting');
    case 'disabled':
      return t('mcp.statusDisabled');
    default:
      return t('mcp.statusStopped');
  }
}

function capabilitySummary(serverId: number) {
  return (
    capabilitySummaryMap.value.get(serverId) ?? {
      tools: [],
      prompts: [],
      resources: [],
      resourceTemplates: [],
    }
  );
}

function runtimeCounts(serverId: number) {
  const summary = capabilitySummary(serverId);
  return {
    tools: summary.tools.length,
    prompts: summary.prompts.length,
    resources: summary.resources.length + summary.resourceTemplates.length,
  };
}

function getToolPayloads(serverId: number): unknown[] {
  return capabilitySummary(serverId).tools;
}

function getPromptPayloads(serverId: number): unknown[] {
  return capabilitySummary(serverId).prompts;
}

function getResourcePayloads(serverId: number): unknown[] {
  return capabilitySummary(serverId).resources;
}

function getResourceTemplatePayloads(serverId: number): unknown[] {
  return capabilitySummary(serverId).resourceTemplates;
}

const expandedMap = ref<Record<number, ExpandKey | null>>({});

function isExpanded(serverId: number, key: ExpandKey): boolean {
  return expandedMap.value[serverId] === key;
}

function toggleExpand(serverId: number, key: ExpandKey): void {
  expandedMap.value[serverId] = expandedMap.value[serverId] === key ? null : key;
}

const disabledItems = ref<Record<string, boolean>>({});
const expandedItems = ref<Record<string, boolean>>({});

function itemKey(serverId: number, type: ExpandKey, index: number): string {
  return `${serverId}-${type}-${index}`;
}

function isItemDisabled(serverId: number, type: ExpandKey, index: number): boolean {
  return disabledItems.value[itemKey(serverId, type, index)] ?? false;
}

function toggleItemDisabled(serverId: number, type: ExpandKey, index: number): void {
  const key = itemKey(serverId, type, index);
  disabledItems.value[key] = !disabledItems.value[key];
}

function isItemExpanded(serverId: number, type: ExpandKey, index: number): boolean {
  return expandedItems.value[itemKey(serverId, type, index)] ?? false;
}

function toggleItemExpanded(serverId: number, type: ExpandKey, index: number): void {
  const key = itemKey(serverId, type, index);
  expandedItems.value[key] = !expandedItems.value[key];
}

function openRunTool(serverId: number, tool: unknown): void {
  saveScrollPosition();
  const obj = tool as Record<string, unknown>;
  runToolServerId.value = serverId;
  runToolName.value = String(obj.name ?? '');
  runToolDescription.value = String(obj.description ?? '');
  runToolParams.value = extractToolParams(tool);
  for (const key of Object.keys(runToolArgs)) {
    delete runToolArgs[key];
  }
  for (const param of runToolParams.value) {
    if (param.defaultValue !== undefined) {
      runToolArgs[param.name] = param.defaultValue;
    } else if (param.type === 'boolean') {
      runToolArgs[param.name] = false;
    } else if (param.type === 'number' || param.type === 'integer') {
      runToolArgs[param.name] = undefined;
    } else {
      runToolArgs[param.name] = '';
    }
  }
  runToolResult.value = null;
  runToolError.value = null;
  runToolLoading.value = false;
  showRunToolView.value = true;
}

function isMultilineParam(param: ParamField): boolean {
  if (param.type === 'object' || param.type === 'array') return true;
  if (param.type === 'string' || param.type === 'any') {
    if (param.description.length > 60) return true;
    const lower = param.description.toLowerCase();
    return ['json', 'content', 'text', 'code', 'body', 'message'].some((v) => lower.includes(v));
  }
  return false;
}

function paramRows(param: ParamField): number {
  if (param.type === 'object') return 6;
  if (param.type === 'array') return 4;
  if (param.type === 'string' && isMultilineParam(param)) return 4;
  return 3;
}

function paramPlaceholder(param: ParamField): string {
  if (param.type === 'object') return '{\n  "key": "value"\n}';
  if (param.type === 'array') return t('mcp.arrayPlaceholder');
  return param.description || '';
}

function closeRunTool(): void {
  showRunToolView.value = false;
  runToolResult.value = null;
  runToolError.value = null;
  restoreScrollPosition();
}

async function executeRunTool(): Promise<void> {
  if (runToolServerId.value === null) return;
  runToolLoading.value = true;
  runToolResult.value = null;
  runToolError.value = null;
  try {
    const args: Record<string, unknown> = {};
    for (const param of runToolParams.value) {
      const val = runToolArgs[param.name];
      if (val === undefined || val === '') continue;
      if (param.type === 'boolean') {
        args[param.name] = Boolean(val);
      } else if (param.type === 'number' || param.type === 'integer') {
        const num = Number(val);
        args[param.name] = Number.isNaN(num) ? val : num;
      } else if (param.type === 'object') {
        if (typeof val === 'string' && val.trim()) {
          try {
            args[param.name] = JSON.parse(val);
          } catch {
            args[param.name] = val;
          }
        }
      } else if (param.type === 'array') {
        if (typeof val === 'string' && val.trim()) {
          args[param.name] = val
            .split('\n')
            .map((line) => line.trim())
            .filter((line) => line !== '');
        }
      } else if (typeof val === 'string') {
        try {
          const parsed = JSON.parse(val);
          args[param.name] = typeof parsed !== 'string' ? parsed : val;
        } catch {
          args[param.name] = val;
        }
      } else {
        args[param.name] = val;
      }
    }

    const result = await store.callTool(runToolServerId.value, runToolName.value, args);
    const textParts: string[] = [];
    for (const item of result.content) {
      const c = item as Record<string, unknown>;
      if (c.type === 'text' && typeof c.text === 'string') {
        textParts.push(c.text);
      } else {
        textParts.push(JSON.stringify(c, null, 2));
      }
    }
    runToolResult.value = textParts.join('\n');
    if (result.isError) {
      runToolError.value = t('mcp.toolRunError');
    }
  } catch (err) {
    runToolError.value = String(err);
  } finally {
    runToolLoading.value = false;
  }
}

function parseJsonToArgs(jsonStr: string): string[] {
  try {
    const arr = JSON.parse(jsonStr || '[]') as unknown[];
    if (!Array.isArray(arr)) return [''];
    const filtered = arr.filter((v): v is string => typeof v === 'string' && v.trim() !== '');
    return filtered.length > 0 ? filtered : [''];
  } catch {
    return [''];
  }
}

function argsToJson(entries: string[]): string {
  const parts: string[] = [];
  for (const entry of entries) {
    const trimmed = entry.trim();
    if (!trimmed) continue;
    parts.push(...trimmed.split(/\s+/));
  }
  return JSON.stringify(parts);
}

function onArgsKeydown(e: KeyboardEvent): void {
  if (e.key !== 'Enter') return;
  const target = e.target as HTMLElement;
  if (target.tagName !== 'INPUT') return;
  if (!target.closest('.n-dynamic-input')) return;
  e.preventDefault();
  argsEntries.value = [...argsEntries.value, ''];
}

function parseJsonToEntries(jsonStr: string): KeyValueEntry[] {
  try {
    const obj = JSON.parse(jsonStr || '{}') as Record<string, string>;
    const entries = Object.entries(obj)
      .filter(([k]) => k.trim() !== '')
      .map(([key, value]) => ({ key, value: String(value) }));
    return entries.length > 0 ? entries : [{ key: '', value: '' }];
  } catch {
    return [{ key: '', value: '' }];
  }
}

function entriesToJson(entries: KeyValueEntry[]): string {
  const obj: Record<string, string> = {};
  for (const entry of entries) {
    if (entry.key.trim()) {
      obj[entry.key.trim()] = entry.value.trim();
    }
  }
  return JSON.stringify(obj);
}

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
  if (e.key !== 'Escape') return;
  if (showFormModal.value) {
    closeForm();
  } else if (showRunToolView.value) {
    closeRunTool();
  } else if (showImportView.value) {
    closeImport();
  }
}

function resetFormLocalState(): void {
  envEntries.value = [{ key: '', value: '' }];
  headerEntries.value = [{ key: '', value: '' }];
  argsEntries.value = [''];
  argsExpanded.value = false;
  envExpanded.value = false;
  headersExpanded.value = false;
}

function openCreate(): void {
  store.resetForm();
  resetFormLocalState();
  saveScrollPosition();
  showFormModal.value = true;
}

function closeForm(): void {
  store.resetForm();
  resetFormLocalState();
  showFormModal.value = false;
  restoreScrollPosition();
}

async function handleSubmit(): Promise<void> {
  store.form.env = entriesToJson(envEntries.value);
  store.form.headers = entriesToJson(headerEntries.value);
  store.form.args = argsToJson(argsEntries.value);
  const wasEditingId = store.editingId;
  const savedName = store.form.name.trim();
  await store.submitForm();
  let targetId: number | null = wasEditingId;
  if (targetId === null) {
    const target = store.servers.find((s) => s.name === savedName);
    targetId = target?.id ?? null;
  }
  showFormModal.value = false;
  resetFormLocalState();
  restoreScrollPosition();
  if (targetId !== null) {
    await store.connectServer(targetId);
    await store.refreshTools(targetId);
  }
}

const togglingIds = ref(new Set<number>());
const refreshingIds = ref(new Set<number>());

async function handleToggle(server: McpServer): Promise<void> {
  togglingIds.value.add(server.id);
  try {
    if (server.enabled) {
      await store.disconnectServer(server.id);
    }
    await store.toggleServer(server.id);
    if (!server.enabled) {
      await store.connectServer(server.id);
    }
  } finally {
    togglingIds.value.delete(server.id);
  }
}

async function handleDelete(serverId: number): Promise<void> {
  await store.deleteServer(serverId);
}

async function handleRefresh(serverId: number): Promise<void> {
  refreshingIds.value.add(serverId);
  try {
    await store.disconnectServer(serverId);
    await store.connectServer(serverId);
  } finally {
    refreshingIds.value.delete(serverId);
  }
}

function editServer(server: McpServer): void {
  store.startEdit(server);
  envEntries.value = parseJsonToEntries(store.form.env || '{}');
  headerEntries.value = parseJsonToEntries(store.form.headers || '{}');
  argsEntries.value = parseJsonToArgs(store.form.args || '[]');
  argsExpanded.value = argsEntries.value.filter(Boolean).length > 1;
  envExpanded.value = envEntries.value.some((e) => e.key.trim() !== '');
  headersExpanded.value = headerEntries.value.some((e) => e.key.trim() !== '');
  saveScrollPosition();
  showFormModal.value = true;
}

function openImport(): void {
  importJson.value = '';
  importResult.value = null;
  mcpHubImportResult.value = null;
  importLoading.value = false;
  saveScrollPosition();
  showImportView.value = true;
}

function closeImport(): void {
  showImportView.value = false;
  importJson.value = '';
  importResult.value = null;
  mcpHubImportResult.value = null;
  importLoading.value = false;
  restoreScrollPosition();
}

async function handleImport(): Promise<void> {
  if (!importJson.value.trim()) return;
  importLoading.value = true;
  importResult.value = null;
  mcpHubImportResult.value = null;
  try {
    // Auto-detect format: if JSON has a "groups" array, treat as MCPHub
    let isMcpHub = false;
    try {
      const parsed = JSON.parse(importJson.value);
      isMcpHub =
        parsed &&
        typeof parsed === 'object' &&
        Array.isArray((parsed as Record<string, unknown>).groups);
    } catch {
      // parse error will be handled by the import function
    }

    if (isMcpHub) {
      const result = await store.importFromMcpHub(importJson.value);
      mcpHubImportResult.value = result;
    } else {
      const result = await store.importServers(importJson.value);
      importResult.value = result;
      await store.fetchServers();
      const connectPromises = result.success.map(async (name) => {
        const server = store.servers.find((s) => s.name === name);
        if (!server) return;
        try {
          await store.connectServer(server.id);
          await store.refreshTools(server.id);
        } catch {
          // ignore auto-connect failure after successful import
        }
      });
      await Promise.all(connectPromises);
      store.error = null;
    }
  } catch {
    importResult.value = { success: [], failed: [], skipped: [] };
  } finally {
    importLoading.value = false;
  }
}
</script>

<template>
  <main class="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-base">
    <PageHeader :title="t('mcp.title')" :description="t('mcp.description')" />

    <div ref="scrollContainer" class="min-h-0 flex-1 overflow-y-auto px-6 pb-24 md:px-8">
      <McpServerList
        v-if="!showFormModal && !showRunToolView && !showImportView"
        :servers="servers"
        :filtered-servers="filteredServers"
        :loading="store.loading"
        :refreshing="store.refreshing"
        :search-query="searchQuery"
        :toggling-ids="togglingIds"
        :refreshing-ids="refreshingIds"
        :status-of="statusOf"
        :transport-label="transportLabel"
        :transport-badge-class="transportBadgeClass"
        :status-badge-class="statusBadgeClass"
        :status-dot-class="statusDotClass"
        :status-text="statusText"
        :runtime-error="(serverId) => store.runtimeFor(serverId).error"
        :runtime-counts="runtimeCounts"
        :get-tool-payloads="getToolPayloads"
        :get-prompt-payloads="getPromptPayloads"
        :get-resource-payloads="getResourcePayloads"
        :get-resource-template-payloads="getResourceTemplatePayloads"
        :is-expanded="isExpanded"
        :toggle-expand="toggleExpand"
        :is-item-disabled="isItemDisabled"
        :toggle-item-disabled="toggleItemDisabled"
        :is-item-expanded="isItemExpanded"
        :toggle-item-expanded="toggleItemExpanded"
        @update:search-query="searchQuery = $event"
        @create="openCreate"
        @import="openImport"
        @refresh-all="store.fetchServers()"
        @edit="editServer"
        @refresh="handleRefresh"
        @toggle="handleToggle"
        @delete="handleDelete"
        @run-tool="openRunTool"
      />

      <McpServerForm
        v-else-if="showFormModal"
        :form="form"
        :is-editing="isEditing"
        :env-expanded="envExpanded"
        :headers-expanded="headersExpanded"
        :args-expanded="argsExpanded"
        :env-entries="envEntries"
        :header-entries="headerEntries"
        :args-entries="argsEntries"
        :error="store.error"
        @close="closeForm"
        @submit="handleSubmit"
        @update:env-expanded="envExpanded = $event"
        @update:headers-expanded="headersExpanded = $event"
        @update:args-expanded="argsExpanded = $event"
        @update:env-entries="envEntries = $event"
        @update:header-entries="headerEntries = $event"
        @update:args-entries="argsEntries = $event"
        @args-keydown="onArgsKeydown"
      />

      <McpToolRunner
        v-else-if="showRunToolView"
        :run-tool-name="runToolName"
        :run-tool-description="runToolDescription"
        :run-tool-params="runToolParams"
        :run-tool-args="runToolArgs"
        :run-tool-loading="runToolLoading"
        :run-tool-result="runToolResult"
        :run-tool-error="runToolError"
        :is-multiline-param="isMultilineParam"
        :param-rows="paramRows"
        :param-placeholder="paramPlaceholder"
        @close="closeRunTool"
        @execute="executeRunTool"
      />

      <McpImportView
        v-else
        :import-json="importJson"
        :import-loading="importLoading"
        :import-result="importResult"
        :mcp-hub-import-result="mcpHubImportResult"
        :error="store.error"
        @close="closeImport"
        @submit="handleImport"
        @update:import-json="importJson = $event"
      />
    </div>

    <div
      v-if="showFormModal || showRunToolView || showImportView"
      class="sticky bottom-0 flex h-14 items-center border-t border-divider-soft bg-base/90 px-6 backdrop-blur-sm md:px-8"
    >
      <div class="flex w-full items-center justify-between">
        <n-button v-if="showFormModal" quaternary size="small" @click="closeForm">
          {{ t('common.cancel') }}
        </n-button>
        <n-button v-else-if="showRunToolView" quaternary size="small" @click="closeRunTool">
          {{ t('common.back') }}
        </n-button>
        <n-button v-else quaternary size="small" :disabled="importLoading" @click="closeImport">
          {{ t('common.cancel') }}
        </n-button>

        <n-button v-if="showFormModal" type="primary" :loading="store.saving" @click="handleSubmit">
          <template #icon><Save class="h-4 w-4" /></template>
          {{ t('common.save') }}
        </n-button>
        <n-button v-else-if="showRunToolView" type="primary" :loading="runToolLoading" @click="executeRunTool">
          <template #icon><Play class="h-4 w-4" /></template>
          {{ t('mcp.execute') }}
        </n-button>
        <n-button
          v-else
          type="primary"
          :loading="importLoading"
          :disabled="!importJson.trim()"
          @click="handleImport"
        >
          {{ t('common.import') }}
        </n-button>
      </div>
    </div>
  </main>
</template>
