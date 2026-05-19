# MCPDock — AI Agent Guide

> A lightweight desktop app for managing MCP (Model Context Protocol) servers.
> Built with **Tauri 2 + Vue 3 + Rust**.

---

## Project Overview

MCPDock lets users visually manage MCP servers, group them, and expose aggregated endpoints via a local Streamable HTTP gateway. Key features:

- **MCP Server Management** — Add STDIO or Streamable HTTP servers, connect/disconnect, discover tools/prompts/resources.
- **Group & Gateway** — Group servers into endpoints at `http://localhost:{port}/mcp/{group_name}` with optional Bearer auth.
- **Settings** — Gateway port, auth, timeout, keep-alive, proxy, language, theme, auto-start.
- **System Tray** — Close-to-tray, single instance, dock icon toggle (macOS).
- **i18n** — Full Chinese (zh-CN) and English support.
- **Auto-update** — Built-in updater via `tauri-plugin-updater`.

---

## Tech Stack

| Layer | Technology |
| --- | --- |
| Desktop Framework | Tauri 2 |
| Backend | Rust, axum, tokio, rmcp, rusqlite (bundled SQLite) |
| Frontend | Vue 3, TypeScript, `<script setup lang="ts">` |
| UI Components | Naive UI |
| Styling | Tailwind CSS 4 (no config file — tokens in `src/style.css` `@theme {}`) |
| State Management | Pinia |
| i18n | vue-i18n |
| Linting / Formatting | Biome |
| Package Manager | pnpm |
| Build | Vite + Rolldown |

---

## Architecture

```
mcpdock/
├── src/                        # Vue 3 frontend
│   ├── components/             # Page & layout components
│   │   ├── AppSidebar.vue      # Navigation sidebar (mcp / group / settings)
│   │   ├── PageHeader.vue      # Reusable page header
│   │   ├── GatewayStatus.vue   # Gateway status indicator
│   │   ├── McpManagement.vue   # MCP server list & management
│   │   ├── GroupManagement.vue # Group management
│   │   ├── SettingsPage.vue    # Application settings
│   │   ├── mcp/                # MCP sub-components
│   │   │   ├── McpServerList.vue
│   │   │   ├── McpServerCard.vue
│   │   │   ├── McpServerForm.vue
│   │   │   ├── McpImportView.vue
│   │   │   ├── McpToolRunner.vue
│   │   │   └── shared.ts
│   │   └── group/              # Group sub-components
│   │       ├── GroupCard.vue
│   │       ├── GroupForm.vue
│   │       └── GroupList.vue
│   ├── stores/                 # Pinia stores
│   │   ├── mcp.ts             # Server state, IPC, runtime events
│   │   ├── group.ts           # Group state, IPC
│   │   ├── settings.ts        # Settings state, theme, locale
│   │   └── updater.ts        # Auto-update state
│   ├── types/                  # Shared TypeScript interfaces
│   │   ├── mcp.ts
│   │   ├── group.ts
│   │   └── settings.ts
│   ├── i18n/                   # vue-i18n setup
│   ├── locales/                # Language packs (en.ts, zh-CN.ts)
│   ├── assets/                 # Static assets + design spec
│   ├── App.vue                 # Root layout (sidebar + page切换)
│   ├── main.ts                 # App entry (Naive UI plugin, Pinia, i18n)
│   └── style.css               # Global styles + Tailwind @theme tokens
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── main.rs             # Entry point
│   │   ├── lib.rs              # Tauri builder: setup, tray, single-instance, gateway
│   │   ├── state.rs            # AppState (DB, runtimes, clients, settings, gateway)
│   │   ├── process_env.rs      # PATH repair for child processes
│   │   ├── commands/           # Tauri IPC command handlers
│   │   │   ├── mcp.rs          # Server CRUD, connect/disconnect, tool call
│   │   │   ├── group.rs        # Group CRUD + gateway restart
│   │   │   ├── settings.rs     # Settings read/write + gateway restart
│   │   │   ├── gateway.rs      # Gateway status query & restart
│   │   │   ├── capability.rs   # Capability listing
│   │   │   ├── import_export.rs # JSON import/export
│   │   │   └── install_mode.rs # Windows portable detection
│   │   ├── mcp/                # MCP client management
│   │   │   ├── runtime.rs      # McpClientHolder, McpServerRuntime types
│   │   │   └── manager/        # Connection lifecycle
│   │   │       ├── mod.rs      # connect, disconnect, refresh, call_tool
│   │   │       ├── transport.rs # STDIO & Streamable HTTP client creation
│   │   │       ├── discovery.rs # Tool/prompt/resource discovery
│   │   │       └── runtime_state.rs # Runtime state & event emission
│   │   ├── gateway/            # Streamable HTTP gateway
│   │   │   ├── server.rs       # Axum server, auth middleware, CORS
│   │   │   └── handler/        # GroupHandler dispatch
│   │   │       ├── mod.rs
│   │   │       ├── tools.rs
│   │   │       ├── prompts.rs
│   │   │       └── resources.rs
│   │   └── db/                 # SQLite data layer
│   │       ├── mod.rs          # Schema init (WAL mode, foreign keys)
│   │       ├── mcp_server.rs   # Server CRUD
│   │       ├── mcp_group.rs    # Group CRUD
│   │       ├── mcp_capability.rs # Capability storage
│   │       ├── app_settings.rs # Key-value settings
│   │       └── error.rs        # DB error type
│   ├── capabilities/          # Tauri permission capabilities
│   ├── icons/                 # App icons (macOS, Windows, tray)
│   └── Cargo.toml
├── biome.json                  # Biome config (lint, format, CSS)
├── vite.config.ts              # Vite + Tauri + Tailwind + chunk splitting
├── tsconfig.json / tsconfig.node.json
└── package.json
```

### Data Flow

```
┌──────────────────────────────────────────────────────────┐
│                    Frontend (Vue 3)                       │
│  Components ──→ Pinia stores ──→ Tauri invoke() IPC       │
│                    ↖ listen() events                      │
└──────────────────────────┬───────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────┐
│                Backend (Rust / Tauri)                     │
│  commands/*.rs ──→ mcp/manager ──→ rmcp client            │
│                 ──→ gateway/server (axum)                │
│                 ──→ db/* (rusqlite)                       │
└──────────────────────────────────────────────────────────┘
```

### Gateway Request Flow

```
Client → POST http://localhost:3000/mcp/my-group
    → Axum router → Auth middleware
    → GroupHandler → parse prefixed tool name → resolve server
    → Get/connect upstream MCP client → forward call → return result
```

---

## Development Commands

```bash
# Install dependencies
pnpm install

# Run in development mode (hot-reload for both frontend & Rust)
pnpm tauri dev

# Build for production
pnpm tauri build

# Build portable (Windows, no installer)
pnpm tauri:build:portable

# Lint & format
pnpm format          # Format with Biome
pnpm format:check    # Check formatting
pnpm lint            # Lint with Biome
pnpm lint:fix        # Lint & auto-fix
pnpm check           # Full Biome check
pnpm check:fix       # Full Biome check & auto-fix

# Type check
vue-tsc --noEmit
```

---

## Frontend Conventions

### Component Structure

- All components use `<script setup lang="ts">` — no Options API.
- No `<style>` blocks — all styling via Tailwind utility classes + `@theme` tokens in `src/style.css`.
- Pages are lazy-loaded via `defineAsyncComponent()` in `App.vue`.
- Feature sub-components live under `src/components/<feature>/` (e.g., `mcp/`, `group/`).

### State Management

- **Pinia stores** (`src/stores/`) are the single source of truth for shared data.
- Stores call Tauri `invoke()` for CRUD and `listen()` for runtime events.
- Local-only state stays in component `ref()` / `reactive()`.
- Never duplicate server data in components — always read from the store.

### Tauri IPC Pattern

Frontend → Backend:
```ts
import { invoke } from '@tauri-apps/api/core';
const servers = await invoke<McpServer[]>('list_mcp_servers');
```

Backend → Frontend (events):
```ts
import { listen } from '@tauri-apps/api/event';
const unlisten = await listen<McpServerRuntime>('mcp:runtime-changed', (event) => {
  // Merge into store
});
```

### Type Organization

- Shared types live in `src/types/<domain>.ts` (e.g., `mcp.ts`, `group.ts`, `settings.ts`).
- Component-local interfaces can stay inline in `<script setup>`.
- Rust structs use `#[serde(rename_all = "camelCase")]` so frontend types stay camelCase.
- Runtime maps from Rust `HashMap<i64, _>` become `Record<number, _>` on the frontend.

### Styling

- **Tailwind CSS 4** via `@tailwindcss/vite` — no `tailwind.config.ts`.
- Design tokens defined in `src/style.css` under `@theme { }`.
- Token classes: `bg-base`, `text-main`, `text-sub`, `border-light`, `shadow-soft`.
- Semantic colors: `bg-success-soft`, `bg-info-soft`, `bg-error-soft`, `bg-warning-soft`.
- Naive UI theme overrides via `NConfigProvider :theme-overrides` in `App.vue`.
- Conditional classes use `:class` with helper functions.

### i18n

- Language packs in `src/locales/` (`en.ts`, `zh-CN.ts`).
- Use `$t('key')` or `t('key')` in templates, `t('key')` in script.
- Add new keys to **both** locale files simultaneously.

---

## Backend Conventions (Rust)

### IPC Commands

- Each domain has its own file: `commands/mcp.rs`, `commands/group.rs`, etc.
- All commands are registered in `lib.rs` via `tauri::generate_handler![]`.
- Error handling: commands return `Result<T, String>` — use `.map_err(|e| e.to_string())` or `format_error_chain()`.
- Lock the `db: Mutex<Connection>` briefly — acquire, use, release. Never hold across `.await`.
- For async commands interacting with MCP clients or gateway, take `State<'_, AppState>`.

### Database

- SQLite via `rusqlite` (bundled), stored at `app_data_dir/app.db`.
- WAL mode + foreign keys enabled on init.
- Schema is in `db/mod.rs::init_db()` — migrations are inline `CREATE TABLE IF NOT EXISTS`.
- `serde_json` is used for JSON columns (`args`, `env`, `headers`, `config`) validated with `json_valid()` CHECK constraints.

### MCP Client / Gateway

- `rmcp` crate provides MCP client & server (Streamable HTTP) transports.
- STDIO servers launch child processes; Streamable HTTP servers use reqwest with system proxy.
- Gateway (`gateway/server.rs`) uses `axum` + `tower-http` CORS.
- Group handler dispatches tool/prompt/resource calls by parsing the prefixed name.
- Gateway auto-restarts when groups or settings change.

### State

- `AppState` in `state.rs` holds: `db`, `runtimes`, `clients`, `settings`, `gateway`, `gateway_error`.
- `clients: tokio::sync::Mutex<HashMap<i64, McpClientHolder>>` — async-safe.
- `db: std::sync::Mutex<Connection>` — sync lock, never hold across `.await`.
- `settings: tokio::sync::RwLock<AppSettings>` — read-heavy, async-safe.

### Clippy Configuration

The project uses pedantic + nursery lints with specific allows (see `Cargo.toml [lints.clippy]`):
- `too-many-lines`, `wildcard-imports`, `doc-markdown`, `module-name-repetitions`, `significant-drop-tightening`, `format-push-string`, `literal-string-with-formatting-args`, `ignored-unit-patterns`, `let-underscore-untyped`, `needless-pass-by-value` — all allowed.

---

## Build & Release

- `pnpm tauri build` produces platform-specific installers.
- `--features portable` produces a portable Windows build (no installer).
- Release profile: LTO, codegen-units=1, opt-level=3, strip=true, panic=abort.

### Platform Notes

- **macOS**: Unsigned — users must `xattr -rd com.apple.quarantine` after install. Dock icon hides when window closes. Tray icon restores window.
- **Windows**: Custom title-bar-less frame. Supports `--autostart` flag for auto-launch.
- **Single instance**: Enforced via `tauri-plugin-single-instance`. Second launch restores existing window.

---

## Key Files Quick Reference

| File | Purpose |
| --- | --- |
| `src/App.vue` | Root layout, Naive UI theme, i18n, sidebar + page切换 |
| `src/stores/mcp.ts` | MCP Pinia store — servers, runtimes, capabilities |
| `src/stores/group.ts` | Group Pinia store — groups, capabilities |
| `src/stores/settings.ts` | Settings store, theme, locale, auto-start |
| `src/stores/updater.ts` | Update check & download state |
| `src/types/mcp.ts` | McpServer, McpServerInput, McpServerRuntime types |
| `src/types/group.ts` | McpGroup, McpGroupInput, McpCapability types |
| `src/types/settings.ts` | AppSettings type + defaults |
| `src-tauri/src/lib.rs` | Tauri app builder, setup, tray, single-instance |
| `src-tauri/src/state.rs` | AppState struct definition |
| `src-tauri/src/commands/mcp.rs` | MCP server IPC commands |
| `src-tauri/src/commands/group.rs` | Group IPC commands |
| `src-tauri/src/commands/settings.rs` | Settings IPC commands |
| `src-tauri/src/commands/gateway.rs` | Gateway status & restart |
| `src-tauri/src/commands/import_export.rs` | JSON data import/export |
| `src-tauri/src/gateway/server.rs` | Axum gateway server, auth, CORS |
| `src-tauri/src/gateway/handler/` | GroupHandler — tools, prompts, resources |
| `src-tauri/src/mcp/manager/` | MCP client lifecycle, transport, discovery |
| `src-tauri/src/db/mod.rs` | SQLite schema init |
| `src/style.css` | Tailwind @theme tokens |
| `vite.config.ts` | Vite + chunk splitting config |
| `biome.json` | Biome lint & format rules |
| `src-tauri/Cargo.toml` | Rust dependencies & lint config |
| `src-tauri/tauri.conf.json` | Tauri app config |

---

## Common Patterns

### Adding a new Tauri IPC command

1. **Rust**: Add command fn in `src-tauri/src/commands/<domain>.rs` with `#[tauri::command]`.
2. **Rust**: Register in `src-tauri/src/lib.rs` `generate_handler![]`.
3. **Frontend**: Add `invoke()` call in the relevant Pinia store.
4. **Frontend**: If it emits events, add `listen()` in the store.
5. **Type**: Ensure TS type matches the Rust struct (mind camelCase serde).

### Adding a new i18n key

1. Add key to `src/locales/en.ts`.
2. Add same key to `src/locales/zh-CN.ts`.
3. Use `t('key')` in component.

### Adding a new SQLite table / column

1. Add `CREATE TABLE IF NOT EXISTS` or `ALTER TABLE` in `src-tauri/src/db/mod.rs::init_db()`.
2. Add CRUD fns in `src-tauri/src/db/<domain>.rs`.
3. Add corresponding command in `commands/`.
4. Add corresponding Pinia store / type on the frontend.

---

## Forbidden Patterns

- **No `<style>` blocks** in `.vue` files — use Tailwind utilities.
- **No hardcoded colors** in templates — use `@theme` tokens.
- **No `any` type** in TypeScript — strict mode is on.
- **No direct DOM manipulation** — use Vue reactivity.
- **No duplicate state** — Pinia store is the single source of truth.
- **No snake_case** in frontend types for Tauri payloads (Rust uses `rename_all = "camelCase"`).
- **No holding `db` lock across `.await`** — it's a `std::sync::Mutex`.
- **No `@ts-ignore`** — use `@ts-expect-error` with a comment.

---

## Testing

- No test framework configured yet (Vitest planned for future).
- For manual testing: `pnpm tauri dev`.

---

<!-- TRELLIS:START -->

# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:

- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
