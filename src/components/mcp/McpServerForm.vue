<script setup lang="ts">
import { ChevronLeft } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { McpServerInput } from '../../types/mcp';
import type { KeyValueEntry } from './shared';

interface Props {
  form: McpServerInput;
  isEditing: boolean;
  envExpanded: boolean;
  headersExpanded: boolean;
  argsExpanded: boolean;
  envEntries: KeyValueEntry[];
  headerEntries: KeyValueEntry[];
  argsEntries: string[];
  error: string | null;
}

defineProps<Props>();

const emit = defineEmits<{
  close: [];
  submit: [];
  'update:envExpanded': [value: boolean];
  'update:headersExpanded': [value: boolean];
  'update:argsExpanded': [value: boolean];
  'update:envEntries': [value: KeyValueEntry[]];
  'update:headerEntries': [value: KeyValueEntry[]];
  'update:argsEntries': [value: string[]];
  argsKeydown: [event: KeyboardEvent];
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

    <section class="max-w-3xl rounded-lg border border-light bg-base p-5 shadow-soft">
      <div class="mb-4">
        <h2 class="text-lg font-semibold text-main">
          {{ isEditing ? t('mcp.editMcp') : t('mcp.newMcp') }}
        </h2>
        <p class="mt-1 text-sm text-sub">
          {{ t('mcp.mcpFormDescription') }}
        </p>
      </div>

      <n-form :show-label="true" label-placement="left" label-width="80" size="small">
        <n-form-item :label="t('mcp.nameLabel')" required>
          <n-input v-model:value="form.name" placeholder="" />
        </n-form-item>

        <n-form-item :label="t('mcp.transportTypeLabel')">
          <n-radio-group v-model:value="form.transportType" size="small">
            <n-radio-button value="stdio">STDIO</n-radio-button>
            <n-radio-button value="streamable_http">HTTP</n-radio-button>
          </n-radio-group>
        </n-form-item>

        <n-form-item v-if="form.transportType === 'stdio'" :label="t('mcp.commandLabel')" required>
          <n-input v-model:value="form.command" placeholder="" />
        </n-form-item>

        <n-form-item v-else label="URL" required>
          <n-input v-model:value="form.url" placeholder="" />
        </n-form-item>

        <n-form-item v-if="form.transportType === 'stdio'" :label="t('mcp.argsLabel')">
          <n-collapse
            :default-expanded-names="argsExpanded ? ['args'] : []"
            @update:expanded-names="(v: string[]) => emit('update:argsExpanded', v.includes('args'))"
          >
            <n-collapse-item name="args" title="">
              <div @keydown="(event) => emit('argsKeydown', event)">
                <n-dynamic-input
                  :value="argsEntries"
                  :min="1"
                  :placeholder="t('mcp.argsPlaceholder')"
                  @update:value="emit('update:argsEntries', $event)"
                />
              </div>
            </n-collapse-item>
          </n-collapse>
        </n-form-item>

        <n-form-item :label="t('mcp.envLabel')">
          <n-collapse
            :default-expanded-names="envExpanded ? ['env'] : []"
            @update:expanded-names="(v: string[]) => emit('update:envExpanded', v.includes('env'))"
          >
            <n-collapse-item name="env" title="">
              <n-dynamic-input
                :value="envEntries"
                preset="pair"
                key-placeholder="key"
                value-placeholder="value"
                :min="0"
                @update:value="emit('update:envEntries', $event)"
              />
            </n-collapse-item>
          </n-collapse>
        </n-form-item>

        <n-form-item :label="t('mcp.headersLabel')">
          <n-collapse
            :default-expanded-names="headersExpanded ? ['headers'] : []"
            @update:expanded-names="(v: string[]) => emit('update:headersExpanded', v.includes('headers'))"
          >
            <n-collapse-item name="headers" title="">
              <n-dynamic-input
                :value="headerEntries"
                preset="pair"
                key-placeholder="key"
                value-placeholder="value"
                :min="0"
                @update:value="emit('update:headerEntries', $event)"
              />
            </n-collapse-item>
          </n-collapse>
        </n-form-item>
      </n-form>

      <n-alert v-if="error" type="error" class="mt-2" :show-icon="true">
        {{ error }}
      </n-alert>
    </section>
  </div>
</template>
