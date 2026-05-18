import type { Composer } from 'vue-i18n';

export interface KeyValueEntry {
  key: string;
  value: string;
}

export type McpStatus = 'running' | 'stopped' | 'disabled' | 'error' | 'connecting';
export type ExpandKey = 'tools' | 'prompts' | 'resources';

export interface ParamField {
  name: string;
  type: string;
  required: boolean;
  description: string;
  enumValues?: string[];
  defaultValue?: unknown;
}

export function transportLabel(transport: 'stdio' | 'streamable_http'): string {
  switch (transport) {
    case 'stdio':
      return 'STDIO';
    case 'streamable_http':
      return 'HTTP';
  }
}

export function itemName(item: unknown, t: Composer['t']): string {
  if (item && typeof item === 'object' && 'name' in item) {
    return String((item as Record<string, unknown>).name ?? t('common.none'));
  }
  return t('common.none');
}

export function itemDescription(item: unknown): string {
  if (item && typeof item === 'object' && 'description' in item) {
    return String((item as Record<string, unknown>).description ?? '');
  }
  return '';
}

export function extractToolParams(tool: unknown): ParamField[] {
  if (!tool || typeof tool !== 'object') return [];
  const obj = tool as Record<string, unknown>;
  const schema = obj.inputSchema;
  if (!schema || typeof schema !== 'object') return [];
  const s = schema as Record<string, unknown>;
  const properties = (s.properties ?? {}) as Record<string, unknown>;
  const required = Array.isArray(s.required) ? (s.required as string[]) : [];
  const fields: ParamField[] = [];
  for (const [key, val] of Object.entries(properties)) {
    if (!val || typeof val !== 'object') continue;
    const prop = val as Record<string, unknown>;
    const baseType = Array.isArray(prop.type)
      ? ((prop.type as string[]).find((t) => t !== 'null') ?? (prop.type as string[])[0])
      : ((prop.type as string) ?? 'any');
    const enumVals = Array.isArray(prop.enum)
      ? (prop.enum as unknown[]).map((v) => String(v))
      : undefined;
    fields.push({
      name: key,
      type: enumVals ? 'enum' : baseType,
      required: required.includes(key),
      description: String(prop.description ?? ''),
      enumValues: enumVals,
      defaultValue: prop.default,
    });
  }
  return fields;
}

export function extractPromptParams(prompt: unknown): ParamField[] {
  if (!prompt || typeof prompt !== 'object') return [];
  const obj = prompt as Record<string, unknown>;
  const args = obj.arguments;
  if (!Array.isArray(args)) return [];
  return (args as Record<string, unknown>[]).map((arg) => ({
    name: String(arg.name ?? ''),
    type: 'string',
    required: arg.required === true,
    description: String(arg.description ?? ''),
  }));
}

export function extractResourceMeta(res: unknown): {
  uri?: string;
  uriTemplate?: string;
  mimeType?: string;
} {
  if (!res || typeof res !== 'object') return {};
  const obj = res as Record<string, unknown>;
  return {
    uri: obj.uri ? String(obj.uri) : undefined,
    uriTemplate: obj.uriTemplate ? String(obj.uriTemplate) : undefined,
    mimeType: obj.mimeType ? String(obj.mimeType) : undefined,
  };
}
