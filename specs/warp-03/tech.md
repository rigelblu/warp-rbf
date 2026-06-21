# Collapse/restore a pane to an edge rail

Status: implemented and dogfood-passed for `#warp-03`. The implementation shipped in feature commit `e415e6b99b63` (`tkwkxtnl`) and these code references were line-checked against current Git HEAD `c43ca7761615070e21ee8c25d7687f04482735fe`. The referenced pane-group and persistence files have no working-copy diff from `c43ca776`. Fork-local links below use `rigelblu/warp-rbf`.

Product spec: `specs/warp-03/product.md`

## Context
`product.md` defines the user-visible contract. The as-built implementation keeps the feature inside the existing pane split tree and render pipeline rather than adding a new layout engine or a global dock.

- [`app/src/pane_group/tree.rs:42-45 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L42-L45) defines the fixed rail and chevron sizes.
- [`app/src/pane_group/tree.rs:66-84 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L66-L84) adds `HiddenPaneReason::Collapsed`, distinct from closed/move/job/child-agent hiding.
- [`app/src/pane_group/tree.rs:358-397 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L358-L397) implements collapse, restore, collapse-order, and the last-visible-pane guard.
- [`app/src/pane_group/tree.rs:470-485 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L470-L485) is the fixed persistence split: closed panes are closed, while collapsed panes are kept in snapshots.
- [`app/src/pane_group/tree.rs:573-588 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L573-L588) excludes collapsed panes from navigation through the generic hidden-pane check.
- [`app/src/pane_group/mod.rs:411-466 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/mod.rs#L411-L466) registers the macOS default bindings.
- [`app/src/pane_group/mod.rs:5711-5815 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/mod.rs#L5711-L5815) handles directional collapse, retract-wins restore, expand-to-edge, and rail-click restore/focus.
- [`app/src/pane_group/tree.rs:1503-1538 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L1503-L1538) implements the structural bordering-group lookup used by collapse.
- [`app/src/pane_group/tree.rs:1170-1203 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L1170-L1203) renders a collapsed leaf or fully collapsed subtree as one rail before normal `Shrinkable` pane rendering.
- [`app/src/pane_group/tree.rs:1592-1618 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L1592-L1618) defines which nodes count as rails for render and resize.
- [`app/src/pane_group/tree.rs:1661-1744 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L1661-L1744) creates the arrow-only clickable rail and dispatches `RestoreCollapsedGroup`.
- [`app/src/pane_group/tree.rs:1270-1343 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L1270-L1343) and [`tree.rs:1422-1452 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree.rs#L1422-L1452) skip rails for mouse and keyboard resize.
- [`app/src/pane_group/mod.rs:2128-2155 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/mod.rs#L2128-L2155) snapshots pane nodes and keeps collapsed panes in the layout.
- [`app/src/app_state.rs:95-127 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/app_state.rs#L95-L127) shows `PaneNodeSnapshot` carries only branch/leaf layout, not collapsed rail state.
- [`app/src/persistence/sqlite.rs:842-865 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/persistence/sqlite.rs#L842-L865), [`sqlite.rs:1047-1085 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/persistence/sqlite.rs#L1047-L1085), and [`sqlite.rs:2120-2131 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/persistence/sqlite.rs#L2120-L2131) persist and restore the snapshot tree.
- [`crates/warpui_core/src/elements/flex/mod.rs:221-245 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/crates/warpui_core/src/elements/flex/mod.rs#L221-L245) is why the rail can be a non-flexible child while sibling panes keep flexing into the remainder.

## Proposed changes
This spec records the as-built implementation.

1. Model rail state as `HiddenPaneReason::Collapsed` on the existing `PaneData.hidden_panes` list. Keep it hidden for navigation, but do not treat it as closed or snapshot-omitted.

2. Keep collapse/restore as tree mutations. `collapse_pane` appends a collapsed marker and refuses hidden panes or the last visible pane; `restore_collapsed_pane` removes only the collapsed marker. The pane never leaves the tree, so original position and session identity survive.

3. Drive user actions through `PaneGroupAction::CollapsePane(Direction)`, `ExpandPaneToEdge`, and `RestoreCollapsedGroup`. Default macOS bindings are editable bindings using `meta-shift-H/J/K/L/E`; no separate restore binding is registered because opposite-key restore is handled by `CollapsePane`.

4. Use `pane_group_by_direction` for collapse instead of `panes_by_direction`. Collapse needs the structural adjacent sibling subtree, not the geometry-filtered focus-navigation target, so group collapse and 2x2 scoping follow the split tree.

5. Implement retract-wins restore in the action handler. Before collapsing in a direction, look in the opposite direction for the nearest railed group; if found, restore it and keep focus on the active pane. Otherwise collapse the nearest expanded group in the requested direction, hopping past already railed groups for progressive collapse.

6. Implement expand-to-edge by collapsing every other visible pane. This reuses the same tree marker and rail render path; it does not introduce a maximize-mode fork.

7. Render rails inside `PaneBranch::render` before normal pane rendering. A collapsed leaf or fully collapsed subtree emits one fixed-size `create_rail(...)` element and consumes that slot's divider. Non-railed panes still render through `Shrinkable`, so the existing Flex algorithm gives the reclaimed space to siblings.

8. Keep rails arrow-only and edge-aware. `create_rail` derives the chevron from `(parent axis, leading/trailing edge)` and dispatches `RestoreCollapsedGroup(ids)` on click; click restore focuses the first restored pane.

9. Resize across rails by resolving dividers to the nearest real panes on each side. Mouse drag and keyboard resize both skip rail nodes; rails keep their fixed size, normal adjacent resize is unchanged, and trailing-edge rail runs have no resize target.

10. Persist pane/session safety without persisting collapsed state. `snapshot_for_node` uses `should_omit_pane_from_snapshot`, which excludes `Collapsed` from omission. `PaneNodeSnapshot` and sqlite restore still have no collapsed-state field, so restart may restore panes expanded; that is the intended v1 tradeoff.

## Testing and validation
Current verification recorded in the feature brief:

- Behavior 1, 4, 5, 8, 9, and 19: `cargo test -p warp --lib pane_group::tree` covers the collapse primitive, navigation/snapshot predicate split, restore-to-position, last-visible-pane no-op, double-collapse/no-op restore, collapse ordering, and structural group lookup. Relevant tests are at [`tree_tests.rs:39-160 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree_tests.rs#L39-L160).
- Behavior 17 and 18: the resize-across-rails unit tests cover mouse/keyboard target resolution across a middle rail, unchanged adjacent resize without rails, and trailing-edge no-target behavior. Relevant tests are at [`tree_tests.rs:828-910 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/tree_tests.rs#L828-L910).
- Behavior 19: `cargo test -p warp --lib test_snapshot_keeps_collapsed_panes_in_layout -- --nocapture` covers app-state snapshot retaining a collapsed pane's layout slot; see [`mod_tests.rs:2125-2150 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/mod_tests.rs#L2125-L2150).
- Behavior 2, 3, 6, 7, 10, 11, 12, 13, 14, 15, and 16: source inspection plus Tom dogfood currently cover the end-to-end keybindings, focus behavior, visual rail orientation, progressive collapse/restore, expand-to-edge, rail click, multi-edge rails, and arrow-only rendering. The feature set dogfood passed on master on 2026-06-21 with `meta-shift-<hjkl>` and `meta-shift-E`; resize-after-collapse dogfood passed on 2026-06-22.
- Behavior 20: source inspection confirms the feature is local pane-layout state and does not introduce network, shell-delivery, or file-access surfaces.

Open validation before treating the manual matrix as fully closed:

1. Re-run the exact Scenario 4 position matrix from the brief: bottom-pane up/down edge behavior, horizontal left/right behavior, and no collision with `cmd-ctrl-<arrow>`. The overall feature set passed dogfood, but that exact matrix is not separately logged.
2. If upstreaming this as an implementation PR, include manual testing proof per `CONTRIBUTING.md`. For this interactive layout feature, a short narrated screen recording is stronger than written notes alone.

## Parallelization
Not proposed. `run_agents` is not available in this environment, and the implemented surface is a tightly coupled pane-tree/render/action/persistence slice. Splitting it across agents would mostly create merge coordination around the same files.

## Risks and mitigations
- **Render/navigation/snapshot predicate drift:** the feature relies on three predicates intentionally disagreeing for `Collapsed`. Mitigation: keep the predicate-split tests and the app-state snapshot test; do not reuse closed-pane predicates for collapsed panes.
- **User keybinding overrides:** the defaults are editable bindings, so stale user overrides can shadow them. Mitigation: document the binding behavior and debug user overrides before blaming macOS option-letter handling.
- **Collapsed-state persistence deferred:** restart may restore a railed pane expanded. Mitigation: the v1 invariant is pane/session survival; full persisted rail state should be a follow-up with an explicit snapshot/schema design.
- **Dogfood-gated rendering:** rail orientation and actual visual feel are not unit-tested in a render harness. Mitigation: keep manual proof attached to an implementation PR and add integration/render coverage if Warp's harness gains a cheap path for this UI state.

## Follow-ups
- Persist collapsed/railed state across app restarts if users need restart to preserve the rail form, not just the pane/session.
- Revisit collapse/restore-from-the-middle in larger 4x4 or 5x5 grids if dogfood shows the current retract-wins rule becomes hard to predict.
