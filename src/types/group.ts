export type McpCapabilityType = 'tool' | 'prompt' | 'resource';

export interface McpCapability {
  id: number;
  serverId: number;
  type: McpCapabilityType;
  capabilityKey: string;
  name: string | null;
  description: string | null;
  payload: unknown;
  updatedAt: string;
}

export interface McpGroupServerSelection {
  serverId: number;
  name: string;
  tools: string[] | null;
  prompts: string[] | null;
  resources: string[] | null;
}

export interface McpGroupConfig {
  servers: McpGroupServerSelection[];
}

export interface McpGroup {
  id: string;
  name: string;
  config: McpGroupConfig;
  createdAt: string;
  updatedAt: string;
}

export interface McpGroupInput {
  name: string;
  config: McpGroupConfig;
}

export function createEmptyGroupInput(): McpGroupInput {
  return {
    name: '',
    config: {
      servers: [],
    },
  };
}
