export interface AppSettings {
  port: number | null;
  proxyUrl: string;
  authEnabled: boolean;
  authToken: string;
  requestTimeoutEnabled: boolean;
  requestTimeoutMs: number | null;
  keepAliveEnabled: boolean;
  keepAliveIntervalMs: number | null;
  gatewaySeparator: string;
  locale: string;
  autoStartEnabled: boolean;
  autoStartHidden: boolean;
  theme: 'light' | 'dark' | 'system';
}

export function defaultSettings(): AppSettings {
  return {
    port: 3100,
    proxyUrl: '',
    authEnabled: false,
    authToken: '',
    requestTimeoutEnabled: true,
    requestTimeoutMs: 60_000,
    keepAliveEnabled: false,
    keepAliveIntervalMs: 60_000,
    gatewaySeparator: '__',
    locale: 'auto',
    autoStartEnabled: false,
    autoStartHidden: false,
    theme: 'system',
  };
}
