export type TransportType = 'stdio' | 'streamable_http';

export interface McpServer {
  id: number;
  name: string;
  enabled: boolean;
  transportType: TransportType;
  command: string | null;
  args: string;
  env: string;
  url: string | null;
  headers: string;
  createdAt: string;
  updatedAt: string;
}

export interface McpServerInput {
  name: string;
  enabled?: boolean;
  transportType: TransportType;
  command?: string;
  args?: string;
  env?: string;
  url?: string;
  headers?: string;
}

export interface McpServerRuntime {
  serverId: number;
  connected: boolean;
  connecting: boolean;
  error?: string;
  discoveredAt?: number;
}
