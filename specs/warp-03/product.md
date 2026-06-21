# Collapse/restore a pane to an edge rail

## Summary
Warp users can tuck an adjacent pane, or a whole bordering pane group, into a thin in-place rail to reclaim space without closing the pane. The shipped feature also lets the focused pane expand to the edge by railing the rest, then restore railed panes with the opposite direction key or by clicking the rail.

## Problem
Split layouts make it easy to keep logs, agents, shells, and references visible, but the only precise way to rebalance space was dragging dividers with the mouse or using full maximize. Users needed a keyboard-first way to park a pane temporarily while keeping its session alive and quickly reversible.

## Goals / Non-goals
Goals:
- Park panes in place as thin edge rails while preserving their sessions.
- Let the focused pane reclaim space without changing tab/window identity or closing panes.
- Preserve resize, focus, and app-state safety around railed panes.

Non-goals:
- No restore-last stack or separate restore binding.
- No rail labels, rotated text, or new renderer text-rotation capability.
- No persisted collapsed/railed state across restart in this slice; restart may restore railed panes expanded.
- No change to the existing full maximize behavior.

## Figma
Figma: none provided. The design source is the committed SVG mockups referenced from the feature brief.

## Behavior
1. When no pane is railed, Warp split panes behave as before: panes render at their flex sizes, divider resize works normally, pane navigation sees every visible pane, and `cmd-shift-enter` maximize remains unchanged.

2. On macOS, the default bindings are `meta-shift-H`, `meta-shift-J`, `meta-shift-K`, and `meta-shift-L` for collapse left/down/up/right respectively, and `meta-shift-E` for expand-to-edge. The collapse bindings are editable Warp keybindings and may be shadowed by existing user overrides just like other editable bindings.

3. A directional collapse key points at the pane or pane group to rail. For example, from a top pane, `meta-shift-J` rails the pane or group below; from a left pane, `meta-shift-L` rails the pane or group to the right.

4. Directional collapse affects the nearest expanded bordering group in the requested direction. If no group exists in that direction, the action is a quiet no-op. Collapsing the last visible pane is also a no-op so a tab always keeps at least one visible pane.

5. A collapsed pane stays in its original layout slot and keeps its session alive. It is excluded from pane navigation while railed, but it is not closed, moved, or removed from the split tree.

6. The focused pane remains focused after keyboard collapse and keyboard restore. It grows into the reclaimed space when a neighbor rails, but the user is still working in the same pane.

7. Rail orientation comes from the collapsed node's parent split, not from the pressed key. A pane in a top/bottom split rails to a thin row; a pane in a left/right split rails to a thin column.

8. If the focused pane borders a structural subtree, one directional collapse rails the whole bordering subtree as one group. A fully collapsed subtree renders as one rail, not as one rail per leaf pane.

9. Collapse is scoped to the focused pane's branch chain. In a row-first 2x2 layout, collapsing right from the top-left pane rails the top-right pane only and leaves the bottom row unchanged; in a column-first 2x2 layout, collapsing across the column boundary rails the whole neighboring column.

10. Directional collapse is progressive. If the immediate group in the requested direction is already railed, the action skips it and rails the next expanded group farther in that direction.

11. The opposite direction key restores the nearest railed group on the side the focused pane's edge would retreat toward. This "retract wins" rule means reversing a collapse restores before the same key family can collapse the other side.

12. Opposite-key restore is progressive too. Repeated opposite-key presses undo a progressive collapse one railed group at a time.

13. Clicking a rail restores that rail's group and moves focus to the first restored pane in that group. Keyboard restore keeps focus in the active pane; rail click follows direct manipulation.

14. `meta-shift-E` expands the focused pane to the edge by railing every other currently visible pane. The focused pane keeps focus, and the railed panes can be restored through the same opposite-key and rail-click mechanics.

15. Multiple rails may exist around one focused pane at the same time, such as one bottom row rail and one right column rail. Rails do not collapse into a single global dock or taskbar.

16. A rail is a fixed 20 px band with a centered 14 px expand chevron. Rails are arrow-only, have no label text, and the chevron points in the direction the pane will grow back from that rail edge.

17. Divider resize continues to work around rails. A rail is not itself resizable and has no divider of its own, but real panes that flank a rail resize across it by mouse drag or `cmd-ctrl-<arrow>` keyboard resize; the rail remains fixed.

18. If a rail run reaches the trailing edge of a branch and no real pane exists beyond it, Warp suppresses the divider for that edge because there is no real resize pair.

19. App-state save/restore must never lose a railed pane or its session. A railed pane remains in the saved layout as live content; because collapsed-state persistence is out of scope, restart may restore that pane expanded.

20. The feature is local layout state only. It does not introduce network behavior, new file access, shell input delivery, modal UI, or a data-sharing surface.
