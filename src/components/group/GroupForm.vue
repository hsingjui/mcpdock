<script setup lang="ts">
import { ChevronLeft, ChevronRight } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { McpCapability } from '../../types/group';
import type { McpServer } from '../../types/mcp';

interface Props {
  servers: McpServer[];
  formName: string;
  isEditing: boolean;
  expandedServers: Record<number, boolean>;
  loading: boolean;
  error: string | null;
  hasServer: (serverId: number) => boolean;
  toggleServer: (server: McpServer, checked: boolean) => void;
  capabilitiesOf: (serverId: number, type: 'tool' | 'prompt' | 'resource') => McpCapability[];
  capabilityLabel: (item: McpCapability) => string;
  capabilityHint: (item: McpCapability) => string;
  statusText: (server: McpServer) => string;
  isCapabilitySelected: (
    serverId: number,
    category: 'tools' | 'prompts' | 'resources',
    key: string,
  ) => boolean;
  toggleCapability: (
    server: McpServer,
    category: 'tools' | 'prompts' | 'resources',
    key: string,
    checked: boolean,
  ) => void;
  setCategoryAll: (server: McpServer, category: 'tools' | 'prompts' | 'resources') => void;
  setCategoryNone: (server: McpServer, category: 'tools' | 'prompts' | 'resources') => void;
}

defineProps<Props>();

const emit = defineEmits<{
  close: [];
  submit: [];
  toggleExpanded: [serverId: number];
  updateName: [value: string];
}>();

const { t } = useI18n();
</script>

<template>
  <div class="mt-4">
    <n-button text size="small" class="mb-3" @click="emit('close')">
      <template #icon>
        <ChevronLeft class="h-4 w-4" />
      </template>
      {{ t('common.backToList') }}
    </n-button>

    <div class="rounded-lg border border-light bg-base p-5 shadow-soft">
      <div class="mb-4">
        <h2 class="text-lg font-semibold text-main">
          {{ isEditing ? t('group.editGroup') : t('group.createGroup') }}
        </h2>
        <p class="mt-1 text-sm text-sub">
          {{ t('group.selectMcp') }}
        </p>
      </div>

      <n-form label-placement="top" :show-feedback="false" class="flex flex-col gap-4">
        <n-form-item :label="t('group.groupNameLabel')" required>
          <div class="flex flex-col">
            <n-input :value="formName" :placeholder="t('group.groupNamePlaceholder')" @update:value="emit('updateName', $event)" />
            <p class="mt-1 text-xs text-sub">
              {{ t('group.groupNameHint') }}
            </p>
          </div>
        </n-form-item>

        <div class="flex flex-col gap-3">
          <h3 class="text-sm font-medium text-main">{{ t('group.mcpServers') }}</h3>
          <div v-for="server in servers" :key="server.id" class="rounded-lg border border-light bg-base">
            <div class="flex items-center justify-between gap-3 px-4 py-3">
              <div class="flex min-w-0 items-center gap-3">
                <n-switch :value="hasServer(server.id)" @update:value="toggleServer(server, $event)" />
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <p class="truncate text-sm font-semibold text-main">
                      {{ server.name }}
                    </p>
                    <n-tag size="small" :type="server.enabled ? 'success' : 'warning'">
                      {{ statusText(server) }}
                    </n-tag>
                  </div>
                </div>
              </div>

              <div class="flex items-center gap-3">
                <span class="text-xs text-sub">
                  {{ t('mcp.toolSlashPromptSlashResource', { tools: capabilitiesOf(server.id, 'tool').length, prompts: capabilitiesOf(server.id, 'prompt').length, resources: capabilitiesOf(server.id, 'resource').length }) }}
                </span>
                <n-button quaternary circle size="small" @click="emit('toggleExpanded', server.id)">
                  <template #icon>
                    <ChevronRight class="h-4 w-4 transition-transform" :class="expandedServers[server.id] ? 'rotate-90' : ''" />
                  </template>
                </n-button>
              </div>
            </div>

            <div v-if="expandedServers[server.id]" class="border-t border-light px-4 py-4">
              <div class="flex flex-col gap-4">
                <section class="flex flex-col gap-2">
                  <div class="flex items-center justify-between gap-3">
                    <h3 class="text-sm font-semibold text-main">{{ t('group.toolSelection') }}</h3>
                    <div class="flex items-center gap-2">
                      <n-button size="small" secondary @click="setCategoryAll(server, 'tools')">{{ t('common.selectAll') }}</n-button>
                      <n-button size="small" secondary @click="setCategoryNone(server, 'tools')">{{ t('common.selectNone') }}</n-button>
                    </div>
                  </div>
                  <div v-if="capabilitiesOf(server.id, 'tool').length === 0" class="text-xs text-sub">
                    {{ t('group.noTools') }}
                  </div>
                  <div v-else class="flex flex-col gap-2">
                    <div
                      v-for="item in capabilitiesOf(server.id, 'tool')"
                      :key="item.id"
                      class="rounded-md border border-light px-3 py-2"
                    >
                      <n-checkbox
                        :checked="isCapabilitySelected(server.id, 'tools', item.capabilityKey)"
                        @update:checked="toggleCapability(server, 'tools', item.capabilityKey, $event)"
                      >
                        <div class="min-w-0">
                          <p class="truncate text-sm font-medium text-main">{{ capabilityLabel(item) }}</p>
                          <p class="mt-0.5 text-xs text-sub">{{ capabilityHint(item) }}</p>
                        </div>
                      </n-checkbox>
                    </div>
                  </div>
                </section>

                <section v-if="capabilitiesOf(server.id, 'prompt').length > 0" class="flex flex-col gap-2">
                  <div class="flex items-center justify-between gap-3">
                    <h3 class="text-sm font-semibold text-main">{{ t('group.promptSelection') }}</h3>
                    <div class="flex items-center gap-2">
                      <n-button size="small" secondary @click="setCategoryAll(server, 'prompts')">{{ t('common.selectAll') }}</n-button>
                      <n-button size="small" secondary @click="setCategoryNone(server, 'prompts')">{{ t('common.selectNone') }}</n-button>
                    </div>
                  </div>
                  <div class="flex flex-col gap-2">
                    <div
                      v-for="item in capabilitiesOf(server.id, 'prompt')"
                      :key="item.id"
                      class="rounded-md border border-light px-3 py-2"
                    >
                      <n-checkbox
                        :checked="isCapabilitySelected(server.id, 'prompts', item.capabilityKey)"
                        @update:checked="toggleCapability(server, 'prompts', item.capabilityKey, $event)"
                      >
                        <div class="min-w-0">
                          <p class="truncate text-sm font-medium text-main">{{ capabilityLabel(item) }}</p>
                          <p class="mt-0.5 text-xs text-sub">{{ capabilityHint(item) }}</p>
                        </div>
                      </n-checkbox>
                    </div>
                  </div>
                </section>

                <section v-if="capabilitiesOf(server.id, 'resource').length > 0" class="flex flex-col gap-2">
                  <div class="flex items-center justify-between gap-3">
                    <h3 class="text-sm font-semibold text-main">{{ t('group.resourceSelection') }}</h3>
                    <div class="flex items-center gap-2">
                      <n-button size="small" secondary @click="setCategoryAll(server, 'resources')">{{ t('common.selectAll') }}</n-button>
                      <n-button size="small" secondary @click="setCategoryNone(server, 'resources')">{{ t('common.selectNone') }}</n-button>
                    </div>
                  </div>
                  <div class="flex flex-col gap-2">
                    <div
                      v-for="item in capabilitiesOf(server.id, 'resource')"
                      :key="item.id"
                      class="rounded-md border border-light px-3 py-2"
                    >
                      <n-checkbox
                        :checked="isCapabilitySelected(server.id, 'resources', item.capabilityKey)"
                        @update:checked="toggleCapability(server, 'resources', item.capabilityKey, $event)"
                      >
                        <div class="min-w-0">
                          <p class="truncate text-sm font-medium text-main">{{ capabilityLabel(item) }}</p>
                          <p class="mt-0.5 text-xs text-sub">{{ capabilityHint(item) }}</p>
                        </div>
                      </n-checkbox>
                    </div>
                  </div>
                </section>
              </div>
            </div>
          </div>
        </div>
      </n-form>
    </div>

    <n-alert v-if="error" type="error" class="mt-3">{{ error }}</n-alert>
  </div>
</template>
