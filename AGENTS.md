# AGENTS.md

## --- Project Overview --------------------------------------------------------

**Zapret Interactive** - a graphical shell based on bol-van's original zapret-win-bundle (DPI bypass utility) with preset settings/strategies and a beautiful, minimalistic interface that allows you to change these settings without having to go into text editors

## --- Tech Stack --------------------------------------------------------------

| Layer | Technology |
|---|---|
| Desktop shell | Tauri v2 |
| Frontend runtime | Bun |
| Build tool | Vite |
| UI framework | React 19 |
| Language | TypeScript (strict) |
| Styling | TailwindCSS v4 |
| Component library | shadcn/ui |
| Routing | TanStack Router |
| State management | Zustand |
| Backend language | Rust (Tauri commands) |
| Window effects | `window-vibrancy` crate |
| Notifications | Sonner (toast) |

## --- Folder Structure --------------------------------------------------------

- Frontend: Feature-Sliced Design (FSD)
- Backend: Vertical Slice Design

## --- Core Priorities ---------------------------------------------------------

1. Performance first.
2. Reliability first.
3. Keep behavior predictable under load and during failures (reconnects, failures, third-party module drops).

If a tradeoff is required, choose correctness and robustness over short-term convenience.

## --- Maintainability ---------------------------------------------------------

Long-term maintainability is a core priority. If you add new functionality, first check if there is shared logic that can be extracted to a separate module. Duplicate logic across multiple files is a code smell and should be avoided. Don't be afraid to change existing code. Don't take shortcuts by just adding local logic to solve a problem.

## --- Dependency & Runtime Rules ----------------------------------------------

### Frontend (src)

- **Runtime:** `bun` only. Never use `npm`, `pnpm`, `node` directly.
- Install packages: `bun add <pkg>`
- Dev packages: `bun add -d <pkg>`
- Run scripts: `bunx <tool>` or `bun run <script>`
- Do **not** commit `package-lock.json` or `pnpm-lock.yaml` — only `bun.lock`
- **Theme colors are mandatory:** any new or changed UI colors must come from the palette documented in `assets/THEME.md`. Do not introduce arbitrary hex/RGB/HSL values unless you are mapping an existing token back to that palette.
- **Window materials & transparency:** Window material and transparency configuration has been completely removed. Agents should not enforce or reference `data-webview-material` or material names (acrylic/mica/tabbed). All styling and layout should only reference theme tokens from `assets/THEME.md` and standard `light`/`dark`/`system` theme selection.

### Backend (src-tauri)

- Add dependencies: `cargo add <crate>` — never manually edit `Cargo.toml` version strings
- When adding a crate with features: `cargo add <crate> --features <feat1>,<feat2>`
- After adding deps, always run `cargo check` to verify the build compiles

### Backend Performance

- When adding or changing Rust code that collects independent data from many items, consider `rayon` mandatory-by-default. Use `rayon` when the work is CPU-heavy or bounded independent IO/status work, such as reading many tweak statuses, scanning many registry values, parsing many files, or building many independent metadata objects.
- Keep `rayon` out of code that depends on strict order, shared mutable state, UI-thread affinity, non-thread-safe COM/Win32 objects, global process settings, service-control sequences, or operations where parallelism can amplify system load or side effects.
- For Tauri commands, do not rely on `rayon` alone for responsiveness. Wrap blocking backend work in `tauri::async_runtime::spawn_blocking`, then use `rayon` inside that blocking task only when the per-item work is independent.
- Prefer a small, direct sequential implementation when the collection is tiny, the operation is already asynchronous, or the added parallelism would make error handling or rollback behavior less predictable.

### Tauri

- Use Tauri v2 APIs — do not use v1 patterns (different plugin system, command registration, etc.)
- Register all commands in `lib.rs` via `tauri::Builder::default().invoke_handler(tauri::generate_handler![...])`
- Use `tauri::command` macro on all public Rust handlers

## --- Codebase Navigation & Intelligence --------------------------------------

**MANDATORY:** All codebase navigation, exploration, symbol discovery, and relationship analysis MUST be performed using `@colbymchenry/codegraph@0.8.0`. Generic file searches or regex greps are discouraged unless searching for raw literal strings not indexed as symbols.

### Why codegraph?
`codegraph` parses the entire codebase (both Rust & TypeScript) to build a semantic graph of functions, components, types, and files. This allows instantly finding usages, definitions, and dependencies across language boundaries without flooding the context window with raw text.

### Usage & Commands
Always bypass the unsafe Node version guard since the codebase uses modern runtimes:

```bash
# 1. Re-index codebase (run after modifying code/files)
$env:CODEGRAPH_ALLOW_UNSAFE_NODE=1; bunx @colbymchenry/codegraph@0.8.0 index

# 2. Query specific symbol (find where functions, types, components are defined/used)
$env:CODEGRAPH_ALLOW_UNSAFE_NODE=1; bunx @colbymchenry/codegraph@0.8.0 query <symbol_name>

# 3. Generate structured Markdown context for specific task or feature
$env:CODEGRAPH_ALLOW_UNSAFE_NODE=1; bunx @colbymchenry/codegraph@0.8.0 context "<feature or task description>"

# 4. View indexing stats and verify graph health
$env:CODEGRAPH_ALLOW_UNSAFE_NODE=1; bunx @colbymchenry/codegraph@0.8.0 status
```

### Workflow Rules for AI Agents:
1. **Explore First:** When starting new task, query relevant symbols or generate context markdown using `codegraph context` instead of manually reading multiple files.
2. **Post-edit Re-indexing:** After creating or modifying files, re-index using the `index` command to keep semantic graph current.
3. **Trace Dependencies:** Use `query` to understand which modules, components, or Tauri commands are impacted before making modifications.

## --- Post-Task Checks --------------------------------------------------------

Run checks by the scope of the change. Do **not** run unrelated frontend or backend checks when the edited files cannot affect that area.

- Frontend changes (`src/**/*.ts`, `src/**/*.tsx`, `src/**/*.css`, routing, stores, UI components): run the frontend checks.
- Backend/Tauri changes (`src-tauri/**/*.rs`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, Tauri config/capabilities): run the backend checks.
- Shared contract changes (`src/lib/types.ts`, Tauri command signatures, generated IPC wrappers, config schema/defaults used by both sides): run both frontend and backend checks.
- Dependency changes: run the checks for the side whose dependency changed. Use `bun add`/`bun add -d` for frontend and `cargo add` for backend.
- Scripts and managed resource tooling (`scripts/**`, `thirdparty/**`, `assets/**` used by build/update pipelines): run the relevant script-specific verification plus any side affected by the script output.
- Documentation-only changes (`README.md`, `AGENTS.md`, changelogs, comments-only docs) do not require typecheck/lint/build checks unless they include executable examples or change project rules that affect commands.
- If scope is ambiguous, choose the smaller relevant check set first. Escalate to both frontend and backend checks only when the change crosses the boundary or the first check points to a cross-area issue.

### Frontend

Order matters — format first so typecheck sees clean code:

```bash
# 1. Fix formatting and lint errors
bun run format
# fallback if script not available:
bunx eslint --fix .

# 2. Type check — must pass with zero errors
bun run typecheck
# fallback:
bunx tsc --noEmit

# 3. Dead-code check (fallow) — must pass with zero issues
bunx fallow --only dead-code

# 4. React Doctor audit (ensure UI health)
bunx react-doctor --full --json-compact
```

> `eslint-stylistic` is used for formatting — it replaces Prettier. `bun run format` runs `eslint --fix`, not a separate formatter.

### Backend

Order matters — fmt before clippy so clippy sees formatted code; check after clippy fix to confirm the build is clean:

```bash
# 1. Format
cargo fmt

# 2. Lint + auto-fix what's fixable
cargo clippy --fix --allow-dirty --allow-staged

# 3. Verify the build compiles cleanly
cargo check
```

## --- Reference Repos ---------------------------------------------------------

- All default zapret strategies are taken from here: https://github.com/StressOzz/Zapret-Manager
- Documentation and binary sources of DNS module are taken from here: https://github.com/DNSCrypt/dnscrypt-proxy
- Documentation and binary sources of TG WS Proxy module are taken from here: https://github.com/valnesfjord/tg-ws-proxy-rs
- App theme and color palette is taken from here: https://github.com/kepano/flexoki

Use these as implementation references when designing module handling, UI Design, and operational stuff.
