<script setup lang="ts">
import { FolderPlus } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { McpGroup, McpGroupServerSelection } from '../../types/group';
import GroupCard from './GroupCard.vue';

interface Props {
  groups: McpGroup[];
  loading: boolean;
  error: string | null;
  copyOptions: Array<{ label: string; key: string }>;
  groupUrl: (group: McpGroup) => string;
  serverEnabled: (serverId: number) => boolean;
  selectedCount: (sel: McpGroupServerSelection, type: 'tool' | 'prompt' | 'resource') => number;
  groupTotalCount: (group: McpGroup, type: 'tool' | 'prompt' | 'resource') => number;
}

defineProps<Props>();

const emit = defineEmits<{
  create: [];
  edit: [groupId: string];
  delete: [groupId: string];
  copy: [group: McpGroup, key: string];
}>();

const { t } = useI18n();
</script>

<template>
  <div class="mt-4 flex flex-col gap-5">
    <div class="flex items-center justify-between gap-3">
      <n-tag size="small" :bordered="false" type="info">
        {{ t('group.groupCount', { count: groups.length }) }}
      </n-tag>
      <n-button type="primary" @click="emit('create')">
        <template #icon>
          <FolderPlus class="h-4 w-4" />
        </template>
        {{ t('group.createGroup') }}
      </n-button>
    </div>

    <n-spin :show="loading">
      <n-empty
        v-if="!loading && groups.length === 0"
        :description="t('group.emptyDescription')"
        class="rounded-lg border border-light bg-base py-12"
      />

      <div v-else class="grid grid-cols-1 gap-6 lg:grid-cols-2 2xl:grid-cols-3">
        <GroupCard
          v-for="group in groups"
          :key="group.id"
          :group="group"
          :copy-options="copyOptions"
          :group-url="groupUrl(group)"
          :enabled-server-count="group.config.servers.filter((s) => serverEnabled(s.serverId)).length"
          :total-servers="group.config.servers.length"
          :server-enabled="serverEnabled"
          :selected-count="selectedCount"
          :group-total-count="groupTotalCount"
          @edit="emit('edit', $event)"
          @delete="emit('delete', $event)"
          @copy="(group, key) => emit('copy', group, key)"
        />
      </div>
    </n-spin>

    <n-alert v-if="error" type="error">{{ error }}</n-alert>
  </div>
</template>
