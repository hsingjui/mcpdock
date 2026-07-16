import { invoke } from '@tauri-apps/api/core';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { i18n, resolveSystemLocale } from '../i18n';
import type { SupportedLocale } from '../locales';
import type { AppSettings } from '../types/settings';
import { defaultSettings } from '../types/settings';

export type ThemeMode = 'light' | 'dark' | 'system';

/** Determine if dark mode should be active for a given theme setting. */
function resolveIsDark(theme: ThemeMode): boolean {
  if (theme === 'dark') return true;
  if (theme === 'light') return false;
  // system
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

/** Apply theme to DOM (with transition animation) and return whether dark is active. */
export function applyThemeToDOM(theme: ThemeMode): boolean {
  const dark = resolveIsDark(theme);
  const el = document.documentElement;

  // Enable transitions only during theme switch, remove after animation completes
  el.classList.add('theme-transitioning');
  el.classList.toggle('dark', dark);
  localStorage.setItem('mcpdock:theme', theme);

  const onDone = () => {
    el.classList.remove('theme-transitioning');
    el.removeEventListener('transitionend', onDone);
    clearTimeout(fallback);
  };
  el.addEventListener('transitionend', onDone);
  // Fallback: remove class even if no transitionend fires (e.g. pref-reduced-motion)
  const fallback = setTimeout(onDone, 400);

  return dark;
}

/** Shared reactive dark-state ref so App.vue can sync Naive theme. */
export const isDarkRef = ref(document.documentElement.classList.contains('dark'));

/** Media query listener for system theme changes. */
let systemMediaHandler: ((e: MediaQueryListEvent) => void) | null = null;
const systemMediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

/** Install or remove the system media query listener based on current theme. */
function syncSystemMediaListener(theme: ThemeMode): void {
  if (systemMediaHandler) {
    systemMediaQuery.removeEventListener('change', systemMediaHandler);
    systemMediaHandler = null;
  }
  if (theme === 'system') {
    systemMediaHandler = (_e: MediaQueryListEvent) => {
      isDarkRef.value = applyThemeToDOM('system');
    };
    systemMediaQuery.addEventListener('change', systemMediaHandler);
  }
}

/** Apply theme to DOM, update isDarkRef, and sync system listener. */
export function applyTheme(theme: ThemeMode): void {
  isDarkRef.value = applyThemeToDOM(theme);
  syncSystemMediaListener(theme);
}

/** Sanitize null numeric fields back to defaults before sending to backend. */
function sanitize(s: AppSettings): AppSettings {
  const defaults = defaultSettings();
  return {
    ...s,
    port: s.port ?? defaults.port,
    requestTimeoutMs: s.requestTimeoutMs ?? defaults.requestTimeoutMs,
    keepAliveIntervalMs: s.keepAliveIntervalMs ?? defaults.keepAliveIntervalMs,
  };
}

async function syncAutostart(enabled: boolean): Promise<void> {
  if (enabled) {
    // Re-register even when already enabled so upgrades repair stale Windows
    // entries that are missing the --autostart argument or point to an old path.
    await enable();
    return;
  }

  const registered = await isEnabled();
  if (registered) {
    await disable();
  }
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>(defaultSettings());
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);

  const hasProxy = computed(() => settings.value.proxyUrl.trim().length > 0);
  const hasAuth = computed(
    () => settings.value.authEnabled && settings.value.authToken.trim().length > 0,
  );

  /** Whether the current form is valid and can be saved. */
  const canSave = computed(() => {
    const s = settings.value;
    if (s.port == null) return false;
    if (s.requestTimeoutEnabled && s.requestTimeoutMs == null) return false;
    if (s.keepAliveEnabled && s.keepAliveIntervalMs == null) return false;
    return true;
  });

  async function fetchSettings(): Promise<void> {
    loading.value = true;
    error.value = null;

    try {
      settings.value = await invoke<AppSettings>('get_app_settings');
      // Resolve locale: if "auto", detect system language
      if (settings.value.locale === 'auto') {
        const resolved = resolveSystemLocale();
        settings.value.locale = resolved;
        // Persist the resolved locale
        try {
          settings.value = await invoke<AppSettings>('update_app_settings', {
            settings: settings.value,
          });
        } catch {
          // Non-critical: locale resolved in-session even if persist fails
        }
      }
      // Sync autostart with OS
      try {
        await syncAutostart(settings.value.autoStartEnabled);
      } catch {
        // Non-critical
      }
      // Sync i18n with the resolved locale
      i18n.global.locale.value = settings.value.locale as SupportedLocale;
      localStorage.setItem('mcpdock:locale', settings.value.locale);
      // Apply theme
      applyTheme(settings.value.theme);
    } catch (reason) {
      error.value = String(reason);
    } finally {
      loading.value = false;
    }
  }

  async function saveSettings(): Promise<void> {
    saving.value = true;
    error.value = null;

    try {
      const payload = sanitize(settings.value);
      settings.value = payload; // reflect sanitized values in UI
      settings.value = await invoke<AppSettings>('update_app_settings', {
        settings: payload,
      });
      // Sync autostart with OS
      try {
        await syncAutostart(settings.value.autoStartEnabled);
      } catch {
        // Non-critical: autostart sync failure should not block save
      }
      // Sync i18n locale after save
      i18n.global.locale.value = settings.value.locale as SupportedLocale;
      localStorage.setItem('mcpdock:locale', settings.value.locale);
      // Apply theme
      applyTheme(settings.value.theme);
    } catch (reason) {
      error.value = String(reason);
      throw reason;
    } finally {
      saving.value = false;
    }
  }

  function resetForm(): void {
    settings.value = defaultSettings();
  }

  return {
    settings,
    loading,
    saving,
    error,
    isDarkRef,
    hasProxy,
    hasAuth,
    canSave,
    fetchSettings,
    saveSettings,
    resetForm,
  };
});
