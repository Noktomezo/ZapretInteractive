# AGENTS.md — mandatory rules for agents

## Rust — STRICT RULES (always on)

These rules apply to **every** Rust change. No exceptions for “quick fix”, “example”, or “temporary”.

### 1. Correctness first
- Code must compile. Do not leave broken code, unfinished stubs that don’t type-check, or “fix later” that breaks the build.
- Prefer `Result` / explicit error types over `unwrap()` / `expect()` in non-test, non-prototype paths.
- `unwrap()` / `expect()` only when the invariant is locally obvious and documented in a one-line comment *why* it cannot fail.
- No silent `let _ = ...` on fallible ops unless the ignore is intentional and named (`let _ = intentionally_ignored`).

### 2. Ownership & borrowing
- Do not fight the borrow checker with `clone()` spam. Clone only when needed; prefer references, `Arc`, or restructuring.
- Prefer `&str` / `&[T]` / `&Path` in APIs over owned types unless ownership transfer is required.
- Avoid circular `Rc`/`Arc` without `Weak`. Prefer clear ownership trees.
- Interior mutability (`RefCell`, `Mutex`, `RwLock`) only when shared mutable state is unavoidable; document why.

### 3. Types & APIs
- Public APIs: explicit types on exported functions when it improves clarity; no “clever” inference that hides intent.
- Prefer enums + exhaustive `match` over boolean flags and stringly-typed states.
- No `as` casts unless necessary; prefer `TryFrom` / `from` / checked conversions. Comment every `as`.
- Avoid `dyn Trait` unless object safety / heterogeneous collections require it. Prefer generics + monomorphization when feasible.
- Do not introduce new dependencies without need. Prefer std, then existing workspace crates.

### 4. Error handling
- Errors must be actionable: context via `anyhow::Context` / `with_context` or domain error types — not bare strings everywhere.
- Libraries: typed errors (`thiserror` or hand-rolled). Binaries/apps: `anyhow` at the edges is fine.
- Never discard errors with empty `ok()` or empty match arms on `Err`.

### 5. Async & concurrency
- Do not block the async runtime (`std::thread::sleep`, heavy sync I/O, CPU-bound work on the async thread).
- CPU-heavy work → dedicated thread / `spawn_blocking` / rayon as appropriate.
- `Send` + `Sync` bounds only when required; don’t sprinkle them “just in case”.
- Prefer structured concurrency: cancel-safe patterns, no detached tasks that own critical state without supervision.

### 6. Unsafe
- **No `unsafe` by default.** Every `unsafe` block needs:
  1. A short `// SAFETY:` comment stating the invariant
  2. Why safe alternatives are insufficient
- Prefer existing safe wrappers over raw pointers / transmute / manual lifetimes in unsafe.

### 7. Style & structure
- Follow `rustfmt` and project `clippy` (pedantic lints only if the repo already enables them).
- Modules: small, cohesive. No god-files. Split when a file mixes unrelated concerns.
- Naming: idiomatic Rust (`snake_case`, `CamelCase`, `SCREAMING_SNAKE` for consts). No Hungarian notation.
- Comments explain *why*, not *what*. Delete narrating comments (`// increment i`).
- No dead code: no unused imports, vars, or `#[allow(dead_code)]` without justification.

### 8. Tests
- Non-trivial logic gets tests. Pure functions → unit tests; integration boundaries → integration tests.
- Tests must be deterministic. No reliance on wall-clock, ordering of HashMap, or network without isolation.
- Prefer table-driven tests for multiple cases.

### 9. Performance (only when relevant)
- No premature micro-optimizations. Measure first if claiming a perf fix.
- Avoid unnecessary allocations in hot paths (`format!` in tight loops, repeated `to_string()`, collect when an iterator suffices).
- Prefer `impl Iterator` / streaming over intermediate `Vec` when the API allows.

### 10. Git & Branching Workflow (STRICT)
- **Feature branch from `main`**: Always branch from `origin/main` for each new task, fix, or feature (e.g., `feature/<name>`, `fix/<name>`). Never accumulate unrelated changes into long-lived omnibus branches (no 80k line monster PRs).
- **PRs directly to `main`**: Open Pull Requests directly from the focused `feature/<name>` branch into `main` for clean review.
- **Local sync**: Keep the user's local working copy updated with the feature changes (via worktree, direct branch switch, or cherry-pick/rebase) so changes are immediately testable locally.
- **Minimal diff**: Change only what is required for the task. Do not reformat unrelated code, rename widely, or perform drive-by edits.
- **No out-of-scope features**: Implement strictly what was requested.

### 11. Mandatory Quality Gate — Full Verification Pipeline (EVERY TASK)
Before marking any task complete, committing, or pushing, you MUST run this verification pipeline in order:

1. **Format**: `cargo fmt --all` (format all source files to project standards).
2. **Type Check**: `cargo check --all-targets` (verify compilation across all targets).
3. **Clippy (Strict Zero-Warning Policy)**: `cargo clippy --all-targets -- -D warnings` (must pass with 0 warnings, fix any lints).
4. **Tests**: `cargo test` (verify all unit and integration tests pass with 0 failures).

Checklist before delivery:
- [ ] `cargo fmt --all` executed
- [ ] `cargo check --all-targets` succeeds
- [ ] `cargo clippy --all-targets -- -D warnings` passes with 0 warnings
- [ ] `cargo test` passes with 0 failures
- [ ] No new unjustified `unwrap` / `unsafe` / deps
- [ ] Errors propagated with context
- [ ] Public API and ownership make sense
- [ ] Diff is minimal and on-task

### Structure & size
- Keep source files **under ~500 lines**; split earlier (~400) when responsibility blurs.
- Layout by layer/feature: `app/`, `ui/`, `domain/`, `services/`, `state/` — not giant catch-all files.
- No god `utils.rs` / bloated `main.rs`. One clear job per file.
- Prefer `feature/mod.rs` + small siblings over one huge module.

### Hard refusals
Refuse or stop and ask when asked to:
- Ship code that doesn’t compile
- Hide errors / swallow `Result`
- Add `unsafe` without invariants
- Pull heavy deps for trivial tasks
- Mass-refactor unrelated modules under a small feature request

### Don’t reinvent the wheel
If a problem is already solved by a **maintained, widely used crate** (or one already in the workspace), use it.
Do not hand-roll HTTP, parsing, crypto, retries, CLI, serialization, etc. without a concrete reason.
Reuse workspace deps first; new deps need a one-line justification. Reinventing requires an explicit “why not crate X”.

### Web/API/Docs Search — Firecrawl only
No built-in web_search/browse. No alternative scrapers.
If Firecrawl is not authenticated → stop and require user login (`firecrawl login` / `FIRECRAWL_API_KEY`).
Never fall back to other search tools.

### GPUI Skills — MANDATORY (non-negotiable)

Before writing, editing, or suggesting ANY GPUI-related code, you MUST:

1. **Identify relevant skills** for the current task (entities, elements, layout, actions, async, overlays, components, etc.).
2. **Read the matching SKILL.md** (and linked reference files) via the skill tooling / file read. Do not rely on memory or prior turns.
3. **Apply the skill rules** in the solution. If a skill contradicts your default approach, the skill wins.
4. **State briefly** which skill(s) you used (e.g. `used: gpui-entity, gpui-layout-and-style`).

### On every iteration
- Re-check skills if the task scope changed (new component type, overlay, async, custom Element, etc.).
- Do not skip this step because “you already know GPUI”.
- Do not invent APIs, patterns, or positioning rules that conflict with the loaded skills.
- If no skill covers the case, say so explicitly and fall back to official GPUI/Zed patterns — still do not guess.

### Refusal conditions
Refuse to implement if you have not loaded at least one relevant skill for the task domain. Load it first, then continue.

### Preferred skill sources (in order)
1. Project-local skills (`.claude/skills`, `.agents/skills`, `skills/`, etc.)
2. `longbridge/gpui-component` skill set
3. Other installed GPUI skills (`cnwzhu/gpui-skills`, etc.)

### Component Design & Single Source of Truth (Component Library Standard)
- **Single Source of Truth**: When building or refining any UI component that can or should be shared (buttons, icon buttons, inputs, toggles, badges, dialogs, cards, headers, etc.), implement it in `src/ui/components/` with the quality, robustness, and API ergonomics of a professional component library.
- **No Ad-Hoc Duplication**: NEVER write ad-hoc, one-off, copy-pasted implementations of buttons, cards, or controls across pages. All pages and features must consume the shared component from `src/ui/components/`.
- **Craftsmanship & Polish**: Every shared component must have:
  - Full animated micro-interactions and smooth color interpolation (`hover_motion` transitions on background, border, text, and icons).
  - Consistent sizing (`Sm`, `Md`, `Lg`), radii, typography, and spacing tokens.
  - Well-defined semantic variants (`Primary`, `Secondary`, `Outline`, `Ghost`, `Destructive`).
  - First-class support for loading/spinner states, prefix/suffix icons, tooltips, and active/disabled states.
