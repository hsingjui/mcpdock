<script setup lang="ts">
import { ChevronLeft } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { ParamField } from './shared';

interface Props {
  runToolName: string;
  runToolDescription: string;
  runToolParams: ParamField[];
  runToolArgs: Record<string, unknown>;
  runToolLoading: boolean;
  runToolResult: string | null;
  runToolError: string | null;
  isMultilineParam: (param: ParamField) => boolean;
  paramRows: (param: ParamField) => number;
  paramPlaceholder: (param: ParamField) => string;
}

defineProps<Props>();

const emit = defineEmits<{
  close: [];
  execute: [];
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
      <h2 class="text-lg font-semibold text-main">{{ t('mcp.runTool') }}</h2>
      <p class="mt-1 text-sm text-sub">{{ t('mcp.runToolDescription') }}</p>
    </div>

    <section class="max-w-3xl rounded-lg border border-light bg-base p-5 shadow-soft">
      <div class="mb-4">
        <h3 class="text-lg font-semibold text-main">{{ runToolName }}</h3>
        <p v-if="runToolDescription" class="mt-1 text-sm text-sub">
          {{ runToolDescription }}
        </p>
      </div>

      <div v-if="runToolParams.length > 0">
        <n-form label-placement="top" :show-feedback="false">
          <n-form-item v-for="param in runToolParams" :key="param.name">
            <template #label>
              <span class="flex items-center gap-1">
                <span>{{ param.name }}</span>
                <span v-if="param.required" class="text-xs text-error">*</span>
              </span>
            </template>

            <n-select
              v-if="param.type === 'enum'"
              v-model:value="runToolArgs[param.name]"
              :options="param.enumValues?.map((v) => ({ label: v, value: v })) ?? []"
              :placeholder="t('mcp.enumPlaceholder')"
              clearable
            />

            <n-switch v-else-if="param.type === 'boolean'" v-model:value="runToolArgs[param.name]" />

            <n-input-number
              v-else-if="param.type === 'number' || param.type === 'integer'"
              v-model:value="runToolArgs[param.name]"
              class="w-full"
            />

            <n-input
              v-else-if="param.type === 'object'"
              v-model:value="runToolArgs[param.name]"
              type="textarea"
              :placeholder="paramPlaceholder(param)"
              :rows="paramRows(param)"
              :autosize="{ minRows: 3, maxRows: 12 }"
              class="font-mono"
            />

            <n-input
              v-else-if="param.type === 'array'"
              v-model:value="runToolArgs[param.name]"
              type="textarea"
              :placeholder="paramPlaceholder(param)"
              :rows="paramRows(param)"
              :autosize="{ minRows: 2, maxRows: 8 }"
            />

            <n-input
              v-else-if="isMultilineParam(param)"
              v-model:value="runToolArgs[param.name]"
              type="textarea"
              :rows="paramRows(param)"
              :autosize="{ minRows: 2, maxRows: 10 }"
            />
            <n-input v-else v-model:value="runToolArgs[param.name]" />

            <template #feedback>
              <span v-if="param.description" class="text-xs text-sub">{{ param.description }}</span>
            </template>
          </n-form-item>
        </n-form>
      </div>
      <div v-else>
        <p class="text-sm text-sub">{{ t('mcp.noParams') }}</p>
      </div>

      <n-alert v-if="runToolError" type="error" class="mt-3" :show-icon="true">
        {{ runToolError }}
      </n-alert>

      <div v-if="runToolResult !== null" class="mt-3">
        <p class="mb-1.5 text-sm font-medium text-main">{{ t('mcp.executionResult') }}</p>
        <div class="max-h-80 overflow-auto rounded-md border border-light bg-surface p-4">
          <pre class="whitespace-pre-wrap break-words font-mono text-xs text-main">{{ runToolResult }}</pre>
        </div>
      </div>
    </section>
  </div>
</template>
