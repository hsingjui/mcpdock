<script setup lang="ts">
import { Copy, FileText, FolderOpen, MessageSquare, Pencil, Trash2, Wrench } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { McpGroup, McpGroupServerSelection } from '../../types/group';

interface Props {
  group: McpGroup;
  copyOptions: Array<{ label: string; key: string }>;
  groupUrl: string;
  enabledServerCount: number;
  totalServers: number;
  serverEnabled: (serverId: number) => boolean;
  selectedCount: (sel: McpGroupServerSelection, type: 'tool' | 'prompt' | 'resource') => number;
  groupTotalCount: (group: McpGroup, type: 'tool' | 'prompt' | 'resource') => number;
}

defineProps<Props>();

const emit = defineEmits<{
  edit: [groupId: string];
  delete: [groupId: string];
  copy: [group: McpGroup, key: string];
}>();

const { t } = useI18n();
</script>

<template>
  <div class="group/card relative flex flex-col rounded-xl border border-light bg-base p-5 transition-all hover:shadow-[0_8px_30px_rgb(0,0,0,0.04)]">
    <div class="mb-4 flex items-start justify-between">
      <div class="flex items-center gap-3">
        <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary-light text-primary">
          <FolderOpen class="h-5 w-5" />
        </div>
        <div class="min-w-0">
          <h3 class="truncate text-lg font-semibold leading-tight text-main">
            {{ group.name }}
          </h3>
        </div>
      </div>
      <div class="flex items-center gap-1 opacity-0 transition-opacity group-hover/card:opacity-100">
        <n-button quaternary size="tiny" @click="emit('edit', group.id)">
          <template #icon>
            <Pencil class="h-4 w-4" />
          </template>
        </n-button>
        <n-popconfirm
          :positive-text="t('common.delete')"
          :negative-text="t('common.cancel')"
          @positive-click="emit('delete', group.id)"
        >
          <template #trigger>
            <n-button quaternary size="tiny" type="error">
              <template #icon>
                <Trash2 class="h-4 w-4" />
              </template>
            </n-button>
          </template>
          {{ t('group.confirmDeleteGroup') }}
        </n-popconfirm>
      </div>
    </div>

    <div class="mb-5 flex items-center justify-between rounded-lg border border-light bg-surface px-3 py-2">
      <div class="flex min-w-0 items-center gap-2">
        <span class="shrink-0 text-xs font-medium text-disabled">URL</span>
        <span class="truncate text-xs font-mono text-main select-all">{{ groupUrl }}</span>
      </div>
      <n-dropdown :options="copyOptions" trigger="click" @select="(key: string) => emit('copy', group, key)">
        <n-button quaternary size="tiny" class="shrink-0">
          <template #icon>
            <Copy class="h-3.5 w-3.5" />
          </template>
        </n-button>
      </n-dropdown>
    </div>

    <div class="flex-1">
      <h4 class="mb-3 text-xs font-medium text-sub">
        {{ t('group.includedInstances', { enabled: enabledServerCount, total: totalServers }) }}
      </h4>
      <div v-if="group.config.servers.length > 0" class="flex flex-wrap gap-2.5">
        <div
          v-for="sel in group.config.servers"
          :key="sel.serverId"
          class="inline-flex items-center gap-2 rounded border border-light bg-base py-1.5 pl-2 pr-2.5 shadow-sm"
          :class="{ 'opacity-60': !serverEnabled(sel.serverId) }"
        >
          <span
            class="h-1.5 w-1.5 shrink-0 rounded-full"
            :class="serverEnabled(sel.serverId) ? 'bg-primary' : 'bg-placeholder'"
          ></span>
          <span class="text-sm font-medium text-main">{{ sel.name }}</span>
          <div
            v-if="
              selectedCount(sel, 'tool') > 0 ||
              selectedCount(sel, 'prompt') > 0 ||
              selectedCount(sel, 'resource') > 0
            "
            class="ml-1 flex items-center gap-1.5 border-l border-light pl-2"
          >
            <span v-if="selectedCount(sel, 'tool') > 0" class="flex items-center gap-0.5 text-xs text-tag-blue-text">
              <Wrench class="h-3 w-3" />
              {{ selectedCount(sel, 'tool') }}
            </span>
            <span v-if="selectedCount(sel, 'prompt') > 0" class="flex items-center gap-0.5 text-xs text-tag-purple-text">
              <MessageSquare class="h-3 w-3" />
              {{ selectedCount(sel, 'prompt') }}
            </span>
            <span v-if="selectedCount(sel, 'resource') > 0" class="flex items-center gap-0.5 text-xs text-tag-emerald-text">
              <FileText class="h-3 w-3" />
              {{ selectedCount(sel, 'resource') }}
            </span>
          </div>
        </div>
      </div>
      <div
        v-else
        class="flex h-16 items-center justify-center rounded-lg border border-dashed border-light bg-hover-bg"
      >
        <span class="text-xs text-disabled">{{ t('group.noInstances') }}</span>
      </div>
    </div>

    <div class="mt-6 flex items-center justify-between border-t border-light pt-4" :class="{ 'opacity-50': group.config.servers.length === 0 }">
      <span class="text-xs text-sub">{{ t('group.totalProvided') }}</span>
      <div class="flex items-center gap-3 text-xs font-medium text-main">
        <span>{{ t('mcp.toolCount', { count: groupTotalCount(group, 'tool') }) }}</span>
        <span class="h-1 w-1 rounded-full bg-placeholder"></span>
        <span>{{ t('mcp.promptCount', { count: groupTotalCount(group, 'prompt') }) }}</span>
        <span class="h-1 w-1 rounded-full bg-placeholder"></span>
        <span>{{ t('mcp.resourceCount', { count: groupTotalCount(group, 'resource') }) }}</span>
      </div>
    </div>
  </div>
</template>
