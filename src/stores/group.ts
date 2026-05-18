import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type {
  McpCapability,
  McpGroup,
  McpGroupInput,
  McpGroupServerSelection,
} from '../types/group';
import { createEmptyGroupInput } from '../types/group';
import type { McpServer } from '../types/mcp';

function cloneSelection(selection: McpGroupServerSelection): McpGroupServerSelection {
  return {
    serverId: selection.serverId,
    name: selection.name,
    tools: selection.tools ? [...selection.tools] : null,
    prompts: selection.prompts ? [...selection.prompts] : null,
    resources: selection.resources ? [...selection.resources] : null,
  };
}

function cloneInput(input: McpGroupInput): McpGroupInput {
  return {
    name: input.name,
    config: {
      servers: input.config.servers.map(cloneSelection),
    },
  };
}

export const useGroupStore = defineStore('group', () => {
  const groups = ref<McpGroup[]>([]);
  const capabilities = ref<McpCapability[]>([]);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const editingId = ref<string | null>(null);
  const form = ref<McpGroupInput>(createEmptyGroupInput());

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

  async function fetchGroups(): Promise<void> {
    loading.value = true;
    error.value = null;

    try {
      groups.value = await invoke<McpGroup[]>('list_mcp_groups');
    } catch (reason) {
      error.value = String(reason);
    } finally {
      loading.value = false;
    }
  }

  async function fetchCapabilities(serverId?: number): Promise<void> {
    loading.value = true;
    error.value = null;

    try {
      capabilities.value = await invoke<McpCapability[]>('list_mcp_capabilities', {
        serverId: serverId ?? null,
      });
    } catch (reason) {
      error.value = String(reason);
    } finally {
      loading.value = false;
    }
  }

  async function createGroup(input: McpGroupInput): Promise<void> {
    saving.value = true;
    error.value = null;

    try {
      await invoke<McpGroup>('create_mcp_group', { input });
      await fetchGroups();
      resetForm();
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    } finally {
      saving.value = false;
    }
  }

  async function updateGroup(id: string, input: McpGroupInput): Promise<void> {
    saving.value = true;
    error.value = null;

    try {
      await invoke<McpGroup>('update_mcp_group', { id, input });
      await fetchGroups();
      resetForm();
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    } finally {
      saving.value = false;
    }
  }

  async function deleteGroup(id: string): Promise<void> {
    error.value = null;

    try {
      await invoke('delete_mcp_group', { id });
      if (editingId.value === id) {
        resetForm();
      }
      await fetchGroups();
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    }
  }

  async function submitForm(): Promise<void> {
    const payload = cloneInput(form.value);
    if (editingId.value === null) {
      await createGroup(payload);
      return;
    }
    await updateGroup(editingId.value, payload);
  }

  function resetForm(): void {
    editingId.value = null;
    form.value = createEmptyGroupInput();
    error.value = null;
  }

  function startEdit(group: McpGroup): void {
    editingId.value = group.id;
    form.value = {
      name: group.name,
      config: {
        servers: group.config.servers.map(cloneSelection),
      },
    };
  }

  function hasServer(serverId: number): boolean {
    return form.value.config.servers.some((item) => item.serverId === serverId);
  }

  function selectionFor(serverId: number): McpGroupServerSelection | undefined {
    return form.value.config.servers.find((item) => item.serverId === serverId);
  }

  function toggleServer(server: McpServer, checked: boolean): void {
    const index = form.value.config.servers.findIndex((item) => item.serverId === server.id);
    if (checked) {
      if (index >= 0) {
        form.value.config.servers[index] = {
          serverId: server.id,
          name: server.name,
          tools: null,
          prompts: null,
          resources: null,
        };
        return;
      }
      form.value.config.servers.push({
        serverId: server.id,
        name: server.name,
        tools: null,
        prompts: null,
        resources: null,
      });
      return;
    }

    if (index >= 0) {
      form.value.config.servers.splice(index, 1);
    }
  }

  function setCategoryAll(server: McpServer, category: 'tools' | 'prompts' | 'resources'): void {
    const selection = selectionFor(server.id);
    if (!selection) {
      return;
    }
    selection[category] = null;
  }

  function setCategoryNone(server: McpServer, category: 'tools' | 'prompts' | 'resources'): void {
    const selection = selectionFor(server.id);
    if (!selection) {
      return;
    }
    selection[category] = [];
  }

  function isServerFullySelected(serverId: number): boolean {
    const selection = selectionFor(serverId);
    return Boolean(
      selection &&
        selection.tools === null &&
        selection.prompts === null &&
        selection.resources === null,
    );
  }

  function isCategoryAll(serverId: number, category: 'tools' | 'prompts' | 'resources'): boolean {
    const selection = selectionFor(serverId);
    return selection?.[category] === null;
  }

  function isCapabilitySelected(
    serverId: number,
    category: 'tools' | 'prompts' | 'resources',
    key: string,
  ): boolean {
    const selection = selectionFor(serverId);
    if (!selection) {
      return false;
    }
    const values = selection[category];
    if (values === null) {
      return true;
    }
    return values.includes(key);
  }

  function toggleCapability(
    server: McpServer,
    category: 'tools' | 'prompts' | 'resources',
    key: string,
    checked: boolean,
  ): void {
    let selection = selectionFor(server.id);
    if (!selection) {
      selection = {
        serverId: server.id,
        name: server.name,
        tools: [],
        prompts: [],
        resources: [],
      };
      form.value.config.servers.push(selection);
    }

    if (selection[category] === null) {
      selection[category] = [];
    }

    const values = selection[category];
    if (!values) {
      return;
    }

    const index = values.indexOf(key);
    if (checked) {
      if (index < 0) {
        values.push(key);
      }
    } else if (index >= 0) {
      values.splice(index, 1);
    }
  }

  return {
    groups,
    capabilities,
    loading,
    saving,
    error,
    editingId,
    form,
    capabilityMap,
    fetchGroups,
    fetchCapabilities,
    createGroup,
    updateGroup,
    deleteGroup,
    submitForm,
    resetForm,
    startEdit,
    hasServer,
    selectionFor,
    toggleServer,
    setCategoryAll,
    setCategoryNone,
    isServerFullySelected,
    isCategoryAll,
    isCapabilitySelected,
    toggleCapability,
  };
});
