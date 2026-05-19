import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import type { McpCapability } from '../types/group';
import type { McpServer, McpServerInput, McpServerRuntime, TransportType } from '../types/mcp';

export interface McpImportResult {
  success: string[];
  failed: { name: string; error: string }[];
  skipped: string[];
}

export interface McpHubImportResult {
  servers: McpImportResult;
  groups: McpImportResult;
}

export interface CallToolResult {
  content: unknown[];
  isError: boolean | null;
}

const RUNTIME_EVENT = 'mcp:runtime-changed';
const SERVERS_CACHE_KEY = 'mcpdock:mcp-servers';

function createDefaultInput(): McpServerInput {
  return {
    name: '',
    enabled: true,
    transportType: 'stdio',
    command: '',
    args: '',
    env: '{}',
    url: '',
    headers: '{}',
  };
}

export const useMcpStore = defineStore('mcp', () => {
  const servers = ref<McpServer[]>(readCachedServers());
  const runtimes = ref<Record<number, McpServerRuntime>>({});
  const capabilities = ref<McpCapability[]>([]);
  const loading = ref(servers.value.length === 0);
  const refreshing = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const listenerStarted = ref(false);
  const form = ref<McpServerInput>(createDefaultInput());
  const editingId = ref<number | null>(null);
  let unlisten: UnlistenFn | null = null;

  const connectedCount = computed(
    () => Object.values(runtimes.value).filter((runtime) => runtime.connected).length,
  );
  const errorCount = computed(
    () => Object.values(runtimes.value).filter((runtime) => Boolean(runtime.error)).length,
  );
  const recentServers = computed(() => servers.value.slice(0, 3));

  const capabilityMap = computed<Record<number, McpCapability[]>>(() => {
    const grouped: Record<number, McpCapability[]> = {};
    for (const item of capabilities.value) {
      if (!grouped[item.serverId]) {
        grouped[item.serverId] = [];
      }
      grouped[item.serverId].push(item);
    }
    return grouped;
  });

  async function fetchServers(): Promise<void> {
    loading.value = servers.value.length === 0;
    refreshing.value = true;
    error.value = null;

    try {
      const serverList = await invoke<McpServer[]>('list_mcp_servers');
      servers.value = serverList;
      writeCachedServers(serverList);
      loading.value = false;

      void invoke<Record<string, McpServerRuntime>>('get_mcp_runtime', { id: null })
        .then((runtimeMap) => {
          applyRuntimeMap(runtimeMap);
        })
        .catch((reason) => {
          error.value = String(reason);
        });

      // Fire-and-forget: capabilities are non-critical, load in background
      fetchCapabilities().catch(() => {});
    } catch (reason) {
      error.value = String(reason);
    } finally {
      loading.value = false;
      refreshing.value = false;
    }
  }

  async function fetchCapabilities(serverId?: number): Promise<void> {
    try {
      const result = await invoke<McpCapability[]>('list_mcp_capabilities', {
        serverId: serverId ?? null,
      });
      if (serverId != null) {
        // Merge: keep other servers' capabilities, replace only the target server's
        const others = capabilities.value.filter((c) => c.serverId !== serverId);
        capabilities.value = [...others, ...result];
      } else {
        capabilities.value = result;
      }
    } catch (reason) {
      // Capabilities fetch failure is non-critical
      // biome-ignore lint/suspicious/noConsole: legitimate error logging for non-critical failures
      console.error('Failed to fetch capabilities:', reason);
    }
  }

  async function createServer(input: McpServerInput): Promise<void> {
    saving.value = true;
    error.value = null;

    try {
      await invoke<McpServer>('create_mcp_server', { input });
      await fetchServers();
      resetForm();
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    } finally {
      saving.value = false;
    }
  }

  async function updateServer(id: number, input: McpServerInput): Promise<void> {
    saving.value = true;
    error.value = null;

    try {
      await invoke<McpServer>('update_mcp_server', { id, input });
      await fetchServers();
      resetForm();
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    } finally {
      saving.value = false;
    }
  }

  async function deleteServer(id: number): Promise<void> {
    error.value = null;

    try {
      await invoke('delete_mcp_server', { id });
      delete runtimes.value[id];
      if (editingId.value === id) {
        resetForm();
      }
      await fetchServers();
      // Refresh groups since the backend removes the deleted server from all group configs
      const { useGroupStore } = await import('./group');
      useGroupStore().fetchGroups();
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    }
  }

  async function toggleServer(id: number): Promise<void> {
    error.value = null;

    try {
      await invoke<McpServer>('toggle_mcp_server', { id });
      await fetchServers();
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    }
  }

  async function connectServer(id: number): Promise<void> {
    error.value = null;

    try {
      const runtime = await invoke<McpServerRuntime>('connect_mcp_server', { id });
      runtimes.value[id] = runtime;
      await fetchCapabilities(id);
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    }
  }

  async function disconnectServer(id: number): Promise<void> {
    error.value = null;

    try {
      const runtime = await invoke<McpServerRuntime>('disconnect_mcp_server', { id });
      runtimes.value[id] = runtime;
      await fetchCapabilities();
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    }
  }

  async function refreshTools(id: number): Promise<void> {
    error.value = null;

    try {
      const runtime = await invoke<McpServerRuntime>('refresh_mcp_tools', { id });
      runtimes.value[id] = runtime;
      await fetchCapabilities(id);
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    }
  }

  function startEdit(server: McpServer): void {
    editingId.value = server.id;
    form.value = {
      name: server.name,
      enabled: server.enabled,
      transportType: server.transportType,
      command: server.command ?? '',
      args: server.args === '[]' ? '' : server.args,
      env: server.env,
      url: server.url ?? '',
      headers: server.headers,
    };
  }

  function resetForm(): void {
    editingId.value = null;
    form.value = createDefaultInput();
    error.value = null;
  }

  async function submitForm(): Promise<void> {
    const payload = normalizeInput(form.value);
    if (editingId.value === null) {
      await createServer(payload);
      return;
    }
    await updateServer(editingId.value, payload);
  }

  async function startListening(): Promise<void> {
    if (listenerStarted.value) {
      return;
    }

    unlisten = await listen<McpServerRuntime>(RUNTIME_EVENT, (event) => {
      const runtime = event.payload;
      runtimes.value[runtime.serverId] = runtime;
      // When a runtime becomes connected, refresh capabilities from DB
      if (runtime.connected) {
        fetchCapabilities(runtime.serverId);
      }
    });
    listenerStarted.value = true;
  }

  function stopListening(): void {
    if (unlisten) {
      void unlisten();
      unlisten = null;
    }
    listenerStarted.value = false;
  }

  function runtimeFor(serverId: number): McpServerRuntime {
    return (
      runtimes.value[serverId] ?? {
        serverId,
        connected: false,
        connecting: false,
        error: undefined,
        discoveredAt: undefined,
      }
    );
  }

  function capabilitiesOf(serverId: number): McpCapability[] {
    return capabilityMap.value[serverId] ?? [];
  }

  function applyRuntimeMap(runtimeMap: Record<string, McpServerRuntime>): void {
    const nextMap: Record<number, McpServerRuntime> = {};
    for (const [serverId, runtime] of Object.entries(runtimeMap)) {
      nextMap[Number(serverId)] = runtime;
    }
    runtimes.value = nextMap;
  }

  async function callTool(
    serverId: number,
    toolName: string,
    arguments_: Record<string, unknown>,
  ): Promise<CallToolResult> {
    const result = await invoke<CallToolResult>('call_mcp_tool', {
      id: serverId,
      toolName,
      arguments: Object.keys(arguments_).length > 0 ? arguments_ : null,
    });
    return result;
  }

  async function importServers(json: string): Promise<McpImportResult> {
    const result: McpImportResult = { success: [], failed: [], skipped: [] };
    let parsed: unknown;
    try {
      parsed = JSON.parse(json);
    } catch {
      throw new Error('Invalid JSON format. Please check your input.');
    }

    if (!parsed || typeof parsed !== 'object' || parsed === null) {
      throw new Error('JSON content must be an object.');
    }

    const root = parsed as Record<string, unknown>;
    // Support both { mcpServers: { ... } } and { ... } (direct map)
    let serversMap: Record<string, unknown>;
    if ('mcpServers' in root && typeof root.mcpServers === 'object' && root.mcpServers !== null) {
      serversMap = root.mcpServers as Record<string, unknown>;
    } else {
      serversMap = root;
    }

    const existingNames = new Set(servers.value.map((s) => s.name));

    for (const [name, serverDef] of Object.entries(serversMap)) {
      if (typeof serverDef !== 'object' || serverDef === null) {
        result.skipped.push(name);
        continue;
      }

      // Skip if already exists
      if (existingNames.has(name)) {
        result.skipped.push(name);
        continue;
      }

      const def = serverDef as Record<string, unknown>;
      try {
        const input = normalizeInput(parseImportedServer(name, def));
        await invoke<McpServer>('create_mcp_server', { input });
        result.success.push(name);
        existingNames.add(name);
      } catch (e) {
        result.failed.push({ name, error: String(e) });
      }
    }

    return result;
  }

  async function importFromMcpHub(json: string): Promise<McpHubImportResult> {
    const result: McpHubImportResult = {
      servers: { success: [], failed: [], skipped: [] },
      groups: { success: [], failed: [], skipped: [] },
    };

    let parsed: unknown;
    try {
      parsed = JSON.parse(json);
    } catch {
      throw new Error('Invalid JSON format.');
    }

    if (!parsed || typeof parsed !== 'object') {
      throw new Error('JSON content must be an object.');
    }

    const root = parsed as Record<string, unknown>;

    // Extract mcpServers (object map)
    const mcpServersRaw =
      root.mcpServers && typeof root.mcpServers === 'object'
        ? (root.mcpServers as Record<string, unknown>)
        : {};

    // Extract groups (array)
    const groupsRaw = Array.isArray(root.groups) ? (root.groups as unknown[]) : [];

    const existingServerNames = new Set(servers.value.map((s) => s.name));
    const nameToIdMap = new Map<string, number>();

    // Build initial name→id map from existing servers
    for (const s of servers.value) {
      nameToIdMap.set(s.name, s.id);
    }

    // Import servers — direct invoke to avoid per-item fetchServers()
    for (const [name, serverDef] of Object.entries(mcpServersRaw)) {
      if (typeof serverDef !== 'object' || serverDef === null) {
        result.servers.skipped.push(name);
        continue;
      }

      if (existingServerNames.has(name)) {
        result.servers.skipped.push(name);
        continue;
      }

      const def = serverDef as Record<string, unknown>;
      try {
        const input = normalizeInput(parseImportedServer(name, def));
        const created = await invoke<McpServer>('create_mcp_server', { input });
        result.servers.success.push(name);
        existingServerNames.add(name);
        nameToIdMap.set(name, created.id);
      } catch (e) {
        result.servers.failed.push({ name, error: String(e) });
      }
    }

    // Import groups — direct invoke to avoid per-item fetchGroups()
    const { useGroupStore } = await import('./group');
    const groupStore = useGroupStore();
    await groupStore.fetchGroups();
    const existingGroupNames = new Set(groupStore.groups.map((g) => g.name));

    for (const groupRaw of groupsRaw) {
      if (typeof groupRaw !== 'object' || groupRaw === null) continue;
      const g = groupRaw as Record<string, unknown>;
      const groupName = typeof g.name === 'string' ? g.name : '';
      if (!groupName) continue;

      if (existingGroupNames.has(groupName)) {
        result.groups.skipped.push(groupName);
        continue;
      }

      try {
        const groupServers = Array.isArray(g.servers) ? (g.servers as unknown[]) : [];
        const serverSelections: {
          serverId: number;
          name: string;
          tools: string[] | null;
          prompts: string[] | null;
          resources: string[] | null;
        }[] = [];

        for (const srv of groupServers) {
          if (typeof srv !== 'object' || srv === null) continue;
          const s = srv as Record<string, unknown>;
          const serverName = typeof s.name === 'string' ? s.name : '';
          if (!serverName) continue;

          const serverId = nameToIdMap.get(serverName);
          if (serverId === undefined) continue;

          const mapCapability = (val: unknown): string[] | null => {
            if (val === 'all' || val === null || val === undefined) return null;
            if (Array.isArray(val)) return val.filter((v): v is string => typeof v === 'string');
            return null;
          };

          serverSelections.push({
            serverId,
            name: serverName,
            tools: mapCapability(s.tools),
            prompts: mapCapability(s.prompts),
            resources: mapCapability(s.resources),
          });
        }

        const input = { name: groupName, config: { servers: serverSelections } };
        await invoke('create_mcp_group', { input });
        result.groups.success.push(groupName);
        existingGroupNames.add(groupName);
      } catch (e) {
        result.groups.failed.push({ name: groupName, error: String(e) });
      }
    }

    // Batch fetch once after all creates
    await fetchServers();
    await groupStore.fetchGroups();

    // Auto-connect successfully imported servers (parallel)
    const connectPromises = result.servers.success.map(async (name) => {
      const server = servers.value.find((s) => s.name === name);
      if (!server) return;
      try {
        await connectServer(server.id);
        await refreshTools(server.id);
      } catch {
        // ignore auto-connect failure
      }
    });
    await Promise.all(connectPromises);
    error.value = null;

    return result;
  }

  return {
    servers,
    runtimes,
    capabilities,
    loading,
    saving,
    refreshing,
    error,
    form,
    editingId,
    connectedCount,
    errorCount,
    recentServers,
    capabilityMap,
    fetchServers,
    fetchCapabilities,
    createServer,
    updateServer,
    deleteServer,
    toggleServer,
    connectServer,
    disconnectServer,
    refreshTools,
    startEdit,
    resetForm,
    submitForm,
    startListening,
    stopListening,
    runtimeFor,
    capabilitiesOf,
    callTool,
    importServers,
    importFromMcpHub,
  };
});

function readCachedServers(): McpServer[] {
  try {
    const raw = localStorage.getItem(SERVERS_CACHE_KEY);
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) ? (parsed as McpServer[]) : [];
  } catch {
    return [];
  }
}

function writeCachedServers(servers: McpServer[]): void {
  try {
    localStorage.setItem(SERVERS_CACHE_KEY, JSON.stringify(servers));
  } catch {
    // ignore cache write failure
  }
}

function normalizeArgs(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed) return '[]';
  try {
    const parsed = JSON.parse(trimmed);
    if (Array.isArray(parsed)) return trimmed;
  } catch {
    // not valid JSON, fall through to whitespace split
  }
  const parts = trimmed.split(/\s+/).filter(Boolean);
  return JSON.stringify(parts);
}

function normalizeInput(input: McpServerInput): McpServerInput {
  const transportType: TransportType = input.transportType;

  return {
    name: input.name.trim(),
    enabled: input.enabled ?? true,
    transportType,
    command: transportType === 'stdio' ? (input.command?.trim() ?? '') : '',
    args: input.args?.trim() || '[]',
    env: input.env?.trim() || '{}',
    url: transportType === 'streamable_http' ? (input.url?.trim() ?? '') : '',
    headers: input.headers?.trim() || '{}',
  };
}

function parseImportedServer(name: string, def: Record<string, unknown>): McpServerInput {
  // Determine transport type
  // If "command" is present → stdio
  // If "type" is "streamable-http" or "sse" or "http" → streamable_http
  // If "url" is present without command → streamable_http
  const type = typeof def.type === 'string' ? def.type.toLowerCase().trim() : '';
  const hasCommand = typeof def.command === 'string' && def.command.trim() !== '';
  const hasUrl = typeof def.url === 'string' && def.url.trim() !== '';

  let transportType: TransportType;
  if (
    type === 'streamable-http' ||
    type === 'streamable_http' ||
    type === 'sse' ||
    type === 'http'
  ) {
    transportType = 'streamable_http';
  } else if (hasCommand) {
    transportType = 'stdio';
  } else if (hasUrl) {
    transportType = 'streamable_http';
  } else {
    throw new Error('Cannot determine transport type: missing command or url');
  }

  const input: McpServerInput = {
    name,
    enabled: true,
    transportType,
    command: undefined,
    args: '[]',
    env: '{}',
    url: undefined,
    headers: '{}',
  };

  if (transportType === 'stdio') {
    input.command = typeof def.command === 'string' ? def.command.trim() : '';
    if (Array.isArray(def.args)) {
      input.args = JSON.stringify(def.args);
    } else if (typeof def.args === 'string') {
      input.args = normalizeArgs(def.args);
    }
    if (typeof def.env === 'object' && def.env !== null) {
      input.env = JSON.stringify(def.env);
    }
  } else {
    input.url = typeof def.url === 'string' ? def.url.trim() : '';
    if (typeof def.headers === 'object' && def.headers !== null) {
      input.headers = JSON.stringify(def.headers);
    }
  }

  return input;
}
