<script setup lang="ts">
import { ChevronLeft } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { McpHubImportResult, McpImportResult } from '../../stores/mcp';

interface Props {
  importJson: string;
  importLoading: boolean;
  importResult: McpImportResult | null;
  mcpHubImportResult: McpHubImportResult | null;
  error: string | null;
}

defineProps<Props>();

const emit = defineEmits<{
  close: [];
  submit: [];
  'update:importJson': [value: string];
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
    <div class="mb-4">
      <h2 class="text-lg font-semibold text-main">{{ t('mcp.importTitle') }}</h2>
      <p class="mt-1 text-sm text-sub">
        {{ t('mcp.importDescription', { code: 'mcpServers' }) }}
      </p>
    </div>

    <section class="max-w-3xl rounded-lg border border-light bg-base p-5 shadow-soft">
      <n-input
        :value="importJson"
        type="textarea"
        :placeholder="`{\n  &quot;mcpServers&quot;: {\n    &quot;my-server&quot;: {\n      &quot;command&quot;: &quot;npx&quot;,\n      &quot;args&quot;: [&quot;-y&quot;, &quot;example-server&quot;]\n    }\n  }\n}`"
        :rows="8"
        :disabled="importLoading"
        class="font-mono"
        @update:value="emit('update:importJson', $event)"
      />

      <!-- MCPDock format result -->
      <div
        v-if="importResult"
        class="mt-3 flex min-h-[72px] flex-col items-start justify-center rounded-lg border border-light bg-base p-4 text-sm"
      >
        <div v-if="importResult.success.length > 0" class="mb-2">
          <span class="font-medium text-tag-emerald-text">
            {{ t('mcp.importSuccess', { count: importResult.success.length }) }}</span
          >
          <span class="ml-1 text-main">{{ importResult.success.join('、') }}</span>
        </div>
        <div v-if="importResult.skipped.length > 0" class="mb-2">
          <span class="font-medium text-tag-amber-text">
            {{ t('mcp.importSkipped', { count: importResult.skipped.length }) }}</span
          >
          <span class="ml-1 text-main">{{ importResult.skipped.join('、') }}</span>
          <span class="ml-1 text-sub">{{ t('mcp.importSkippedReason') }}</span>
        </div>
        <div v-if="importResult.failed.length > 0" class="text-left">
          <span class="font-medium text-tag-red-text">
            {{ t('mcp.importFailed', { count: importResult.failed.length }) }}</span
          >
          <ul class="mt-1 ml-4 list-disc">
            <li v-for="f in importResult.failed" :key="f.name" class="text-main">
              {{ f.name }}: {{ f.error }}
            </li>
          </ul>
        </div>
        <n-alert
          v-if="
            importResult.success.length === 0 &&
            importResult.failed.length === 0 &&
            importResult.skipped.length === 0
          "
          type="warning"
          class="mt-1"
          :show-icon="true"
        >
          {{ t('mcp.importEmpty') }}
        </n-alert>
      </div>

      <!-- MCPHub format result -->
      <div
        v-if="mcpHubImportResult"
        class="mt-3 flex min-h-[72px] flex-col items-start justify-center rounded-lg border border-light bg-base p-4 text-sm"
      >
        <!-- Servers -->
        <div v-if="mcpHubImportResult.servers.success.length > 0" class="mb-2">
          <span class="font-medium text-tag-emerald-text">
            {{ t('mcp.importSuccess', { count: mcpHubImportResult.servers.success.length }) }}</span
          >
          <span class="ml-1 text-main">{{ mcpHubImportResult.servers.success.join('、') }}</span>
        </div>
        <div v-if="mcpHubImportResult.servers.skipped.length > 0" class="mb-2">
          <span class="font-medium text-tag-amber-text">
            {{ t('mcp.importSkipped', { count: mcpHubImportResult.servers.skipped.length }) }}</span
          >
          <span class="ml-1 text-main">{{ mcpHubImportResult.servers.skipped.join('、') }}</span>
          <span class="ml-1 text-sub">{{ t('mcp.importSkippedReason') }}</span>
        </div>
        <div v-if="mcpHubImportResult.servers.failed.length > 0" class="mb-2 text-left">
          <span class="font-medium text-tag-red-text">
            {{ t('mcp.importFailed', { count: mcpHubImportResult.servers.failed.length }) }}</span
          >
          <ul class="mt-1 ml-4 list-disc">
            <li v-for="f in mcpHubImportResult.servers.failed" :key="f.name" class="text-main">
              {{ f.name }}: {{ f.error }}
            </li>
          </ul>
        </div>

        <!-- Groups -->
        <div v-if="mcpHubImportResult.groups.success.length > 0" class="mb-2">
          <span class="font-medium text-tag-emerald-text">
            {{ t('mcp.importGroupSuccess', { count: mcpHubImportResult.groups.success.length }) }}</span
          >
          <span class="ml-1 text-main">{{ mcpHubImportResult.groups.success.join('、') }}</span>
        </div>
        <div v-if="mcpHubImportResult.groups.skipped.length > 0" class="mb-2">
          <span class="font-medium text-tag-amber-text">
            {{ t('mcp.importGroupSkipped', { count: mcpHubImportResult.groups.skipped.length }) }}</span
          >
          <span class="ml-1 text-main">{{ mcpHubImportResult.groups.skipped.join('、') }}</span>
        </div>
        <div v-if="mcpHubImportResult.groups.failed.length > 0" class="text-left">
          <span class="font-medium text-tag-red-text">
            {{ t('mcp.importGroupFailed', { count: mcpHubImportResult.groups.failed.length }) }}</span
          >
          <ul class="mt-1 ml-4 list-disc">
            <li v-for="f in mcpHubImportResult.groups.failed" :key="f.name" class="text-main">
              {{ f.name }}: {{ f.error }}
            </li>
          </ul>
        </div>

        <n-alert
          v-if="
            mcpHubImportResult.servers.success.length === 0 &&
            mcpHubImportResult.servers.failed.length === 0 &&
            mcpHubImportResult.servers.skipped.length === 0 &&
            mcpHubImportResult.groups.success.length === 0 &&
            mcpHubImportResult.groups.failed.length === 0 &&
            mcpHubImportResult.groups.skipped.length === 0
          "
          type="warning"
          class="mt-1"
          :show-icon="true"
        >
          {{ t('mcp.importEmpty') }}
        </n-alert>
      </div>

      <n-alert v-if="error" type="error" :show-icon="true" class="mt-3">
        {{ error }}
      </n-alert>
    </section>
  </div>
</template>
