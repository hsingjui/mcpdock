<script setup lang="ts">
import { ChevronRight, FileText, MessageSquare, Play, Wrench } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { McpServer } from '../../types/mcp';
import {
  type ExpandKey,
  extractPromptParams,
  extractResourceMeta,
  extractToolParams,
  itemDescription,
  itemName,
  type McpStatus,
} from './shared';

interface RuntimeCounts {
  tools: number;
  prompts: number;
  resources: number;
}

interface Props {
  server: McpServer;
  index: number;
  total: number;
  status: McpStatus;
  runtimeError?: string;
  runtimeCounts: RuntimeCounts;
  transportLabel: string;
  transportBadgeClass: string;
  statusBadgeClass: string;
  statusDotClass: string;
  statusText: string;
  toolPayloads: unknown[];
  promptPayloads: unknown[];
  resourcePayloads: unknown[];
  resourceTemplatePayloads: unknown[];
  refreshing: boolean;
  toggling: boolean;
  isExpanded: (serverId: number, key: ExpandKey) => boolean;
  toggleExpand: (serverId: number, key: ExpandKey) => void;
  isItemDisabled: (serverId: number, type: ExpandKey, index: number) => boolean;
  toggleItemDisabled: (serverId: number, type: ExpandKey, index: number) => void;
  isItemExpanded: (serverId: number, type: ExpandKey, index: number) => boolean;
  toggleItemExpanded: (serverId: number, type: ExpandKey, index: number) => void;
}

defineProps<Props>();

const emit = defineEmits<{
  edit: [server: McpServer];
  refresh: [serverId: number];
  toggle: [server: McpServer];
  delete: [serverId: number];
  runTool: [serverId: number, tool: unknown];
}>();

const { t } = useI18n();
</script>

<template>
  <div
    class="mcp-server-card p-4 transition-colors hover:bg-hover-bg"
    :class="index < total - 1 ? 'border-b border-light' : ''"
  >
    <div class="flex items-center justify-between gap-4">
      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-center gap-2">
          <p
            class="truncate text-xl font-semibold leading-7 text-main"
            :class="{ 'opacity-40': !server.enabled }"
          >
            {{ server.name }}
          </p>
          <span
            class="inline-flex items-center rounded-lg px-2 py-0.5 text-xs font-medium"
            :class="transportBadgeClass"
          >
            {{ transportLabel }}
          </span>
          <n-popover
            v-if="status === 'error'"
            :delay="300"
            placement="bottom"
            :width="320"
            trigger="hover"
          >
            <template #trigger>
              <span
                class="inline-flex items-center gap-1 rounded-lg border px-2 py-0.5 text-xs"
                :class="statusBadgeClass"
              >
                <span
                  class="inline-block h-1.5 w-1.5 rounded-full"
                  :class="statusDotClass"
                ></span>
                {{ statusText }}
              </span>
            </template>
            <p class="text-xs break-all text-tag-red-text">
              {{ runtimeError }}
            </p>
          </n-popover>
          <span
            v-else
            class="inline-flex items-center gap-1 rounded-lg border px-2 py-0.5 text-xs"
            :class="statusBadgeClass"
          >
            <span
              class="inline-block h-1.5 w-1.5 rounded-full"
              :class="statusDotClass"
            ></span>
            {{ statusText }}
          </span>
        </div>

        <div class="mt-2 flex flex-wrap items-center gap-2">
          <n-button
            v-if="status === 'running' && runtimeCounts.tools > 0"
            quaternary
            size="small"
            class="!rounded-lg !bg-tag-blue-bg !px-2 !py-1 !text-xs !font-medium !text-tag-blue-text hover:!opacity-80"
            @click="toggleExpand(server.id, 'tools')"
          >
            <template #icon>
              <Wrench class="h-3.5 w-3.5" />
            </template>
            {{ t('mcp.toolCount', { count: runtimeCounts.tools }) }}
          </n-button>
          <n-button
            v-if="status === 'running' && runtimeCounts.prompts > 0"
            quaternary
            size="small"
            class="!rounded-lg !bg-tag-purple-bg !px-2 !py-1 !text-xs !font-medium !text-tag-purple-text hover:!opacity-80"
            @click="toggleExpand(server.id, 'prompts')"
          >
            <template #icon>
              <MessageSquare class="h-3.5 w-3.5" />
            </template>
            {{ t('mcp.promptCount', { count: runtimeCounts.prompts }) }}
          </n-button>
          <n-button
            v-if="status === 'running' && runtimeCounts.resources > 0"
            quaternary
            size="small"
            class="!rounded-lg !bg-tag-emerald-bg !px-2 !py-1 !text-xs !font-medium !text-tag-emerald-text hover:!opacity-80"
            @click="toggleExpand(server.id, 'resources')"
          >
            <template #icon>
              <FileText class="h-3.5 w-3.5" />
            </template>
            {{ t('mcp.resourceCount', { count: runtimeCounts.resources }) }}
          </n-button>
        </div>
      </div>

      <div class="flex shrink-0 items-center gap-2">
        <n-button-group>
          <n-button
            size="small"
            class="min-w-[68px] justify-center"
            :disabled="refreshing"
            @click="emit('refresh', server.id)"
          >
            {{ t('common.refresh') }}
          </n-button>
          <n-button
            size="small"
            class="min-w-[68px] justify-center"
            @click="emit('edit', server)"
          >
            {{ t('common.edit') }}
          </n-button>
        </n-button-group>

        <n-button-group>
          <n-button
            size="small"
            class="min-w-[68px] justify-center"
            :disabled="toggling"
            @click="emit('toggle', server)"
          >
            {{ server.enabled ? t('common.disabled') : t('common.enabled') }}
          </n-button>
          <n-popconfirm
            :positive-text="t('common.delete')"
            :negative-text="t('common.cancel')"
            @positive-click="emit('delete', server.id)"
          >
            <template #trigger>
              <n-button
                size="small"
                type="error"
                secondary
                class="min-w-[68px] justify-center"
              >
                {{ t('common.delete') }}
              </n-button>
            </template>
            {{ t('mcp.confirmDeleteMcp', { name: server.name }) }}
          </n-popconfirm>
        </n-button-group>
      </div>
    </div>

    <n-collapse
      v-if="isExpanded(server.id, 'tools')"
      class="mt-3"
      :default-expanded-names="['tools']"
    >
      <n-collapse-item name="tools">
        <template #header>
          <span class="text-xs font-semibold text-main">{{ t('mcp.tools') }}</span>
        </template>
        <div v-if="toolPayloads.length === 0" class="text-xs text-sub">
          {{ t('common.none') }}
        </div>
        <div v-else class="flex flex-col gap-2">
          <div
            v-for="(tool, tidx) in toolPayloads"
            :key="tidx"
            class="rounded-lg border border-light bg-base"
            :class="isItemDisabled(server.id, 'tools', tidx) ? 'opacity-50 bg-tag-gray-bg' : ''"
          >
            <div
              class="flex cursor-pointer items-center justify-between gap-2 p-2"
              @click="toggleItemExpanded(server.id, 'tools', tidx)"
            >
              <div class="flex min-w-0 items-center gap-1.5">
                <ChevronRight
                  class="h-3.5 w-3.5 shrink-0 text-sub transition-transform"
                  :class="isItemExpanded(server.id, 'tools', tidx) ? 'rotate-90' : ''"
                />
                <p class="truncate text-sm font-semibold leading-tight text-main">
                  {{ itemName(tool, t) }}
                </p>
              </div>
              <div class="flex shrink-0 items-center gap-1.5">
                <n-button
                  quaternary
                  circle
                  size="small"
                  class="!text-info hover:!bg-info-soft"
                  :title="t('mcp.runTool')"
                  @click.stop="emit('runTool', server.id, tool)"
                >
                  <template #icon>
                    <Play class="h-3.5 w-3.5" />
                  </template>
                </n-button>
                <n-switch
                  :value="!isItemDisabled(server.id, 'tools', tidx)"
                  size="small"
                  @click.stop
                  @update:value="toggleItemDisabled(server.id, 'tools', tidx)"
                />
              </div>
            </div>
            <div
              v-if="isItemExpanded(server.id, 'tools', tidx)"
              class="border-t border-light px-2 pb-2 pt-1.5"
            >
              <p v-if="itemDescription(tool)" class="mb-1.5 text-xs leading-4 text-sub">
                {{ itemDescription(tool) }}
              </p>
              <div v-if="extractToolParams(tool).length > 0">
                <n-table size="small" :single-line="false" class="text-xs">
                  <thead>
                    <tr class="text-left text-sub">
                      <th class="font-medium">{{ t('mcp.paramName') }}</th>
                      <th class="font-medium">{{ t('mcp.paramType') }}</th>
                      <th class="font-medium">{{ t('mcp.paramRequired') }}</th>
                      <th class="font-medium">{{ t('mcp.paramDescription') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="field in extractToolParams(tool)" :key="field.name">
                      <td class="font-mono text-tag-blue-text">{{ field.name }}</td>
                      <td>
                        <span
                          class="inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium"
                          :class="{
                            'bg-tag-blue-bg text-tag-blue-text': field.type === 'string' || field.type === 'enum',
                            'bg-tag-emerald-bg text-tag-emerald-text': field.type === 'number' || field.type === 'integer',
                            'bg-tag-amber-bg text-tag-amber-text': field.type === 'boolean',
                            'bg-tag-purple-bg text-tag-purple-text': field.type === 'object' || field.type === 'array',
                            'bg-tag-gray-bg text-tag-gray-text': field.type === 'any',
                          }"
                        >
                          {{ field.type }}
                        </span>
                        <span
                          v-if="field.enumValues && field.enumValues.length > 0"
                          class="ml-1 text-xs text-sub"
                        >
                          ({{ field.enumValues.join(', ') }})
                        </span>
                      </td>
                      <td>
                        <span v-if="field.required" class="text-error">{{ t('mcp.paramRequiredYes') }}</span>
                        <span v-else class="text-disabled">{{ t('mcp.paramRequiredNo') }}</span>
                      </td>
                      <td class="text-sub">{{ field.description || '-' }}</td>
                    </tr>
                  </tbody>
                </n-table>
              </div>
              <p v-else-if="!itemDescription(tool)" class="text-xs text-disabled">
                {{ t('mcp.noParamRequirement') }}
              </p>
            </div>
          </div>
        </div>
      </n-collapse-item>
    </n-collapse>

    <n-collapse
      v-if="isExpanded(server.id, 'prompts')"
      class="mt-3"
      :default-expanded-names="['prompts']"
    >
      <n-collapse-item name="prompts">
        <template #header>
          <span class="text-xs font-semibold text-main">{{ t('mcp.prompts') }}</span>
        </template>
        <div v-if="promptPayloads.length === 0" class="text-xs text-sub">
          {{ t('common.none') }}
        </div>
        <div v-else class="flex flex-col gap-2">
          <div
            v-for="(prompt, pidx) in promptPayloads"
            :key="pidx"
            class="rounded-lg border border-light bg-base"
            :class="isItemDisabled(server.id, 'prompts', pidx) ? 'opacity-50 bg-tag-gray-bg' : ''"
          >
            <div
              class="flex cursor-pointer items-center justify-between gap-2 p-2"
              @click="toggleItemExpanded(server.id, 'prompts', pidx)"
            >
              <div class="flex min-w-0 items-center gap-1.5">
                <ChevronRight
                  class="h-3.5 w-3.5 shrink-0 text-sub transition-transform"
                  :class="isItemExpanded(server.id, 'prompts', pidx) ? 'rotate-90' : ''"
                />
                <p class="truncate text-sm font-semibold leading-tight text-main">
                  {{ itemName(prompt, t) }}
                </p>
              </div>
              <div class="flex shrink-0 items-center">
                <n-switch
                  :value="!isItemDisabled(server.id, 'prompts', pidx)"
                  size="small"
                  @click.stop
                  @update:value="toggleItemDisabled(server.id, 'prompts', pidx)"
                />
              </div>
            </div>
            <div
              v-if="isItemExpanded(server.id, 'prompts', pidx)"
              class="border-t border-light px-2 pb-2 pt-1.5"
            >
              <p v-if="itemDescription(prompt)" class="mb-1.5 text-xs leading-4 text-sub">
                {{ itemDescription(prompt) }}
              </p>
              <div v-if="extractPromptParams(prompt).length > 0">
                <n-table size="small" :single-line="false" class="text-xs">
                  <thead>
                    <tr class="text-left text-sub">
                      <th class="font-medium">{{ t('mcp.paramName') }}</th>
                      <th class="font-medium">{{ t('mcp.paramRequired') }}</th>
                      <th class="font-medium">{{ t('mcp.paramDescription') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="field in extractPromptParams(prompt)" :key="field.name">
                      <td class="font-mono text-tag-blue-text">{{ field.name }}</td>
                      <td>
                        <span v-if="field.required" class="text-error">{{ t('mcp.paramRequiredYes') }}</span>
                        <span v-else class="text-disabled">{{ t('mcp.paramRequiredNo') }}</span>
                      </td>
                      <td class="text-sub">{{ field.description || '-' }}</td>
                    </tr>
                  </tbody>
                </n-table>
              </div>
              <p v-else-if="!itemDescription(prompt)" class="text-xs text-disabled">
                {{ t('mcp.noParamRequirement') }}
              </p>
            </div>
          </div>
        </div>
      </n-collapse-item>
    </n-collapse>

    <n-collapse
      v-if="isExpanded(server.id, 'resources')"
      class="mt-3"
      :default-expanded-names="['resources']"
    >
      <n-collapse-item name="resources">
        <template #header>
          <span class="text-xs font-semibold text-main">{{ t('mcp.resources') }}</span>
        </template>
        <div
          v-if="resourcePayloads.length === 0 && resourceTemplatePayloads.length === 0"
          class="text-xs text-sub"
        >
          {{ t('common.none') }}
        </div>
        <div v-else class="flex flex-col gap-2">
          <div
            v-for="(res, ridx) in resourcePayloads"
            :key="`resource-${ridx}`"
            class="rounded-lg border border-light bg-base"
            :class="isItemDisabled(server.id, 'resources', ridx) ? 'opacity-50 bg-tag-gray-bg' : ''"
          >
            <div
              class="flex cursor-pointer items-center justify-between gap-2 p-2"
              @click="toggleItemExpanded(server.id, 'resources', ridx)"
            >
              <div class="flex min-w-0 items-center gap-1.5">
                <ChevronRight
                  class="h-3.5 w-3.5 shrink-0 text-sub transition-transform"
                  :class="isItemExpanded(server.id, 'resources', ridx) ? 'rotate-90' : ''"
                />
                <p class="truncate text-sm font-semibold leading-tight text-main">
                  {{ itemName(res, t) }}
                </p>
              </div>
              <div class="flex shrink-0 items-center">
                <n-switch
                  :value="!isItemDisabled(server.id, 'resources', ridx)"
                  size="small"
                  @click.stop
                  @update:value="toggleItemDisabled(server.id, 'resources', ridx)"
                />
              </div>
            </div>
            <div
              v-if="isItemExpanded(server.id, 'resources', ridx)"
              class="border-t border-light px-2 pb-2 pt-1.5"
            >
              <p v-if="itemDescription(res)" class="mb-1.5 text-xs leading-4 text-sub">
                {{ itemDescription(res) }}
              </p>
              <div v-if="extractResourceMeta(res).uri || extractResourceMeta(res).mimeType" class="flex flex-wrap gap-x-3 gap-y-0.5 text-xs">
                <span v-if="extractResourceMeta(res).uri" class="text-sub">
                  <span class="font-medium text-main">URI:</span>
                  <span class="ml-1 font-mono text-tag-blue-text">{{ extractResourceMeta(res).uri }}</span>
                </span>
                <span v-if="extractResourceMeta(res).mimeType" class="text-sub">
                  <span class="font-medium text-main">MIME:</span>
                  <span class="ml-1 text-tag-purple-text">{{ extractResourceMeta(res).mimeType }}</span>
                </span>
              </div>
              <p v-else-if="!itemDescription(res)" class="text-xs text-disabled">
                {{ t('mcp.noMoreMeta') }}
              </p>
            </div>
          </div>

          <div
            v-for="(res, rtidx) in resourceTemplatePayloads"
            :key="`template-${rtidx}`"
            class="rounded-lg border border-light bg-base"
            :class="isItemDisabled(server.id, 'resources', resourcePayloads.length + rtidx) ? 'opacity-50 bg-tag-gray-bg' : ''"
          >
            <div
              class="flex cursor-pointer items-center justify-between gap-2 p-2"
              @click="toggleItemExpanded(server.id, 'resources', resourcePayloads.length + rtidx)"
            >
              <div class="flex min-w-0 items-center gap-1.5">
                <ChevronRight
                  class="h-3.5 w-3.5 shrink-0 text-sub transition-transform"
                  :class="isItemExpanded(server.id, 'resources', resourcePayloads.length + rtidx) ? 'rotate-90' : ''"
                />
                <p class="truncate text-sm font-semibold leading-tight text-main">
                  {{ itemName(res, t) }}
                </p>
              </div>
              <div class="flex shrink-0 items-center">
                <n-switch
                  :value="!isItemDisabled(server.id, 'resources', resourcePayloads.length + rtidx)"
                  size="small"
                  @click.stop
                  @update:value="toggleItemDisabled(server.id, 'resources', resourcePayloads.length + rtidx)"
                />
              </div>
            </div>
            <div
              v-if="isItemExpanded(server.id, 'resources', resourcePayloads.length + rtidx)"
              class="border-t border-light px-2 pb-2 pt-1.5"
            >
              <p v-if="itemDescription(res)" class="mb-1.5 text-xs leading-4 text-sub">
                {{ itemDescription(res) }}
              </p>
              <div
                v-if="extractResourceMeta(res).uriTemplate || extractResourceMeta(res).mimeType"
                class="flex flex-wrap gap-x-3 gap-y-0.5 text-xs"
              >
                <span v-if="extractResourceMeta(res).uriTemplate" class="text-sub">
                  <span class="font-medium text-main">template:</span>
                  <span class="ml-1 font-mono text-tag-blue-text">{{ extractResourceMeta(res).uriTemplate }}</span>
                </span>
                <span v-if="extractResourceMeta(res).mimeType" class="text-sub">
                  <span class="font-medium text-main">MIME:</span>
                  <span class="ml-1 text-tag-purple-text">{{ extractResourceMeta(res).mimeType }}</span>
                </span>
              </div>
              <p v-else-if="!itemDescription(res)" class="text-xs text-disabled">
                {{ t('mcp.noMoreMeta') }}
              </p>
            </div>
          </div>
        </div>
      </n-collapse-item>
    </n-collapse>
  </div>
</template>
