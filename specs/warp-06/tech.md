# Name a window

Status: as-built. The `#warp-06` macOS slice is implemented, and Tom dogfood passed on 2026-06-23 for command feel, macOS title surfaces, restart/session restore, clear behavior, and no literal `/name-window` PTY leak in real recognized-command use.

Code references are pinned to current git SHA `c43ca7761615070e21ee8c25d7687f04482735fe` (`git rev-parse HEAD` in this checkout on 2026-06-25). That SHA is not on a local `origin/*` remote-tracking branch, so links use the `rigelblu/warp-rbf` fork remote.

Product spec: [product.md](product.md)

## Context

[product.md](product.md) defines the user-visible behavior. The implementation fits existing Warp patterns: static slash commands dispatch workspace actions, workspace state owns window-level UI state, `update_window_title` remains the title chokepoint, and app-state persistence owns window snapshots.

- [`commands.rs:149-156 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/search/slash_command_menu/static_commands/commands.rs#L149-L156) defines `/name-window` as an always-available static slash command with a required `<name | --clear>` argument.
- [`commands.rs:655-662 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/search/slash_command_menu/static_commands/commands.rs#L655-L662) registers it next to tab-rename commands.
- [`slash_commands/mod.rs:130-146 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/input/slash_commands/mod.rs#L130-L146) parses the command argument into set, clear, or usage-error outcomes.
- [`slash_commands/mod.rs:654-665 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/input/slash_commands/mod.rs#L654-L665) dispatches set and reset workspace actions from the recognized slash-command handler.
- [`slash_commands/mod.rs:1400-1406 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/input/slash_commands/mod.rs#L1400-L1406) is the shared successful-command cleanup that clears the invoking input unless the command came from a queued prompt resend.
- [`action.rs:147-148 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/action.rs#L147-L148) defines the window-name workspace actions, and [`action.rs:868-894 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/action.rs#L868-L894) marks them as app-state-saving actions.
- [`view.rs:662-668 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view.rs#L662-L668) centralizes custom-title precedence, whitespace handling, and truncation.
- [`view.rs:3281-3289 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view.rs#L3281-L3289) seeds restored workspaces from `WindowSnapshot.custom_title`.
- [`view.rs:5367-5388 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view.rs#L5367-L5388) resolves, pushes, sets, and resets the OS window title.
- [`view.rs:11384-11388 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view.rs#L11384-L11388) snapshots the custom title with the rest of window state.
- [`sqlite.rs:896-898 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/persistence/sqlite.rs#L896-L898) writes `windows.custom_title`, and [`sqlite.rs:2581-2584 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/persistence/sqlite.rs#L2581-L2584) restores it into `WindowSnapshot`.
- [`app_state.rs:45-48 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/app_state.rs#L45-L48), [`model.rs:327-331 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/crates/persistence/src/model.rs#L327-L331), and [`schema.rs:438-442 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/crates/persistence/src/schema.rs#L438-L442) carry the nullable window-level field through app and persistence models.
- [`2026-06-23-000000_add_custom_title_to_windows/up.sql @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/crates/persistence/migrations/2026-06-23-000000_add_custom_title_to_windows/up.sql#L1) adds the local sqlite column.

Warp `CONTRIBUTING.md` matters for validation: implementation PRs need manual testing proof, and user-facing flows should have `crates/integration/` coverage whenever the harness can exercise them.

## Proposed changes

The as-built shape is the desired implementation.

1. Keep the product surface slash-command only. `/name-window` is discoverable through the static slash-command registry and requires an argument. Do not add a macOS Window-menu item, Chrome-style dialog, titlebar editor, settings surface, shell alias, injected stdin path, or PTY escape hatch.

2. Parse the command argument at the slash-command boundary. Missing or all-whitespace arguments return usage feedback. Exact trimmed `--clear` maps to clear. Any other nonblank argument maps to set after trimming surrounding whitespace, preserving internal spaces.

3. Dispatch workspace actions for mutation. `WorkspaceAction::SetActiveWindowName(String)` and `WorkspaceAction::ResetActiveWindowName` keep window-title state in the workspace layer, and `should_save_app_state_on_action` opts both actions into existing app-state saves.

4. Store one optional value per window. `Workspace::custom_title: Option<String>` is the live state, `WindowSnapshot.custom_title` is the app-state boundary, and `windows.custom_title` is the persisted sqlite field. A nullable column is enough because the feature has no history, sync, or multi-value semantics.

5. Resolve titles through the existing chokepoint. `resolved_window_title(custom_title, tab_title)` trims custom names, ignores blank custom values, falls back to the active tab when unset, and applies the existing `MAX_WINDOW_TITLE_LENGTH` truncation before `ctx.windows().set_window_title(...)`.

6. Restore before title updates matter. Workspace construction reads the restored `WindowSnapshot.custom_title` into `Workspace::custom_title`, so later `update_window_title` calls use the saved custom name instead of briefly reverting to the active tab title.

7. Keep the no-PTY contract scoped to recognized command execution. Successful `/name-window` set and clear flows return handled and reach the shared slash-command input cleanup. If the user dismisses slash-command handling and submits literal text as terminal input, that is existing terminal behavior and not a `/name-window` execution path.

8. Keep the mechanism cross-platform but the proof macOS-scoped. The title sink already flows through existing platform window-title plumbing; this slice proves macOS title bar, Window menu, Mission Control, app switcher, tab-switch stability, restart, and clear behavior. Windows/Linux promotion remains `#warp-20`.

## Testing and validation

Current local proof:

- Behaviors 2, 3, 6, and 7: `cargo test -p warp name_window --lib` passed focused parser and command-registration coverage, including `name_window_command_requires_argument`, `name_window_argument_sets_trimmed_name`, `name_window_argument_clears_only_on_exact_clear_flag`, and `name_window_argument_rejects_missing_or_blank_name`.
- Behaviors 1, 4, 5, and 8: `cargo test -p warp custom_title --lib` passed `resolved_window_title_prefers_nonblank_custom_title`, covering fallback, custom-title precedence, trim, blank fallback, and truncation.
- Behaviors 3, 6, and 10: `cargo test -p warp custom_title --lib` also passed `test_name_window_action_set_blank_and_reset` and `test_sqlite_round_trips_window_custom_title`, covering workspace mutation, blank set no-op behavior, reset, and sqlite round-trip.
- Behavior 10 restore path: source inspection verifies restored `WindowSnapshot.custom_title` seeds `Workspace::custom_title` before title recomputation.
- Behaviors 2 and 11 recognized-command no-PTY path: source inspection verifies recognized `/name-window` dispatch returns handled and reaches the shared input-buffer clear. Tom dogfood passed no literal `/name-window` PTY leak in real recognized-command use.
- Behaviors 4, 5, 9, and 10 OS/user proof: Tom dogfood passed on macOS title bar, Window menu, Mission Control/app switcher, tab-switch stability, restart/session restore, clear behavior, and per-window feel.
- Behaviors 12 and 13 product-surface boundary: source inspection verifies this slice adds only the static slash command, workspace actions, title resolution, and persistence plumbing. It does not add a modal, menu item, settings surface, titlebar editor, pointer-only path, or raw-input fallback.
- Build and hygiene already recorded in the brief: `cargo check -p warp` passed, and `git diff --check` passed for the implementation slice.

Required before an upstream implementation PR:

1. Add `crates/integration/` coverage for Behaviors 2 and 11 if the harness can exercise the slash input path. Suggested flow: boot a terminal pane, type `/name-window Integration Window`, press Enter, assert the input clears, assert the shell/block list did not receive the literal command, assert the title state is `Integration Window`, then run `/name-window --clear` and assert the override is unset or the title derives from the active tab again.

2. If raw-mode/TUI ownership prevents reliable integration coverage for the exact no-PTY case, document that harness limit in the PR and attach manual proof. For this interactive feature, a narrated screen recording is stronger than written notes alone.

3. Keep the focused unit tests. They pin cheap parser, action, title-resolution, save-trigger, and persistence regressions even if an integration test covers the user flow.

4. Before review, run the expected upstream command set: `cargo nextest run` or a justified focused equivalent, `./script/format --check`, clippy per `CONTRIBUTING.md`, and `./script/presubmit` before pushing a real upstream PR.

## Parallelization

Not proposed. The implementation is one tightly coupled vertical slice: command registration, command parsing, workspace actions, title resolution, snapshot persistence, and OS title behavior must agree on one field and one contract. Parallel agents would add coordination overhead without reducing meaningful wall-clock time.

## Risks and mitigations

- **No-PTY behavior lacks integration proof:** source inspection and Tom dogfood passed, but upstream review should prefer `crates/integration/` coverage if the harness can exercise the slash input path.
- **Title clobber on tab switch:** mitigated by resolving custom-title precedence inside `update_window_title`, the existing chokepoint for OS title pushes.
- **Restore timing regression:** mitigated by seeding `Workspace::custom_title` from restored snapshots and keeping sqlite round-trip coverage.
- **Product-surface creep:** mitigated by keeping this slice slash-command-only and explicitly excluding menu/dialog/titlebar/settings surfaces.
