# macOS Speak Selection reads the selection

Status: implemented and dogfood-passed for the `#warp-02` macOS slice. Code references are pinned to Git HEAD `c43ca7761615070e21ee8c25d7687f04482735fe`; links use the fork remote because this work is fork-local.

Product spec: `specs/warp-02/product.md`

## Context
`product.md` defines the behavior. The implementation is a narrow macOS accessibility bridge over Warp's existing selected-text policy, not a new selection model.

- [`crates/warpui_core/src/core/view/mod.rs:148-164 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/crates/warpui_core/src/core/view/mod.rs#L148-L164) defines `View::accessibility_data` and the shared `AccessibilityData { content, selected_text }` payload.
- [`app/src/pane_group/mod.rs:2463-2494 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/pane_group/mod.rs#L2463-L2494) is the existing focused-pane selected-text policy: input-editor selected text wins, then terminal selected text is the fallback.
- [`app/src/terminal/view.rs:14513-14535 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/view.rs#L14513-L14535) exposes terminal selected text and input-editor selected text as separate helpers.
- [`app/src/terminal/view.rs:28083-28138 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/view.rs#L28083-L28138) builds the accessibility transcript and selected text for the focused terminal view.
- [`app/src/terminal/model/terminal_model.rs:1727-1740 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/model/terminal_model.rs#L1727-L1740) routes selected text through alt-screen selection when active, otherwise through block-list selection.
- [`app/src/terminal/model/alt_screen.rs:250-272 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/model/alt_screen.rs#L250-L272) reads alt-screen selections with `RespectObfuscatedSecrets::Yes`.
- [`app/src/terminal/model/blocks/selection.rs:880-982 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/model/blocks/selection.rs#L880-L982) expands regular block selections, normalizes reversed order, collects rich-content selected text, and joins selected chunks.
- [`app/src/terminal/model/blocks/selection.rs:984-1033 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/model/blocks/selection.rs#L984-L1033) handles rectangular selections by row.
- [`app/src/terminal/view.rs:19916-19946 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/view.rs#L19916-L19946) centralizes selected-text clearing so stale model selection is not left behind.
- [`crates/warpui/src/platform/mac/window.rs:1310-1340 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/crates/warpui/src/platform/mac/window.rs#L1310-L1340) exposes accessibility contents and selected text over the macOS FFI.
- [`crates/warpui/src/platform/mac/objc/host_view.m:394-409 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/crates/warpui/src/platform/mac/objc/host_view.m#L394-L409) implements `-accessibilitySelectedText`, returns `nil` for an empty selected string, and keeps `accessibilityNumberOfCharacters` at `0`.
- [`app/src/terminal/view_tests.rs:2693-2736 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/terminal/view_tests.rs#L2693-L2736) verifies the input-selection precedence in `TerminalView::accessibility_data`.

## Proposed changes
This section records the as-built implementation.

1. Add `selected_text: Option<String>` to `AccessibilityData`. `View::accessibility_data` continues to default to `None`, so views that do not participate in selected-text accessibility are unchanged.

2. Keep the terminal's broad accessibility transcript behavior intact. `TerminalView::accessibility_data` still returns recent block content plus input text, or full alt-screen output when alt-screen is active, as the fallback `content` value.

3. Populate `AccessibilityData::selected_text` from the terminal's selected-text policy. `selected_text_from_input(ctx)` is checked first; if it is empty, the code falls back to `TerminalModel::selection_to_string(...)` and filters empty strings. This satisfies product Behavior 2 while keeping terminal output behavior aligned with copy for Behavior 3-8, 9, 10, and 14.

4. Reuse `TerminalModel::selection_to_string` instead of reading grids directly in the accessibility path. That preserves existing alt-screen delegation, block-list ordering, rectangular selection, rich-content selection, and secret-obfuscation behavior. The AX path does not invent a second text-selection algorithm.

5. Expose selected text to macOS through the same focused-view accessibility callback as the transcript. `warp_get_accessibility_selected_text` resolves the window id, asks the app for focused-view accessibility data, reads `data.selected_text.unwrap_or_default()`, and returns an autoreleased `NSString`.

6. Implement `-accessibilitySelectedText` on `WarpHostView`. The Obj-C method queries Rust live on every call and returns `nil` when the string is empty, preventing stale empty strings from being treated as a real selection. The spike established that macOS 26.4.1 Speak Selection works with `accessibilitySelectedText` alone, so no selected-text range, string-for-range, or nonzero character-count method shipped.

7. Do not add new invalidation state. Existing selected-text clearing remains the source of truth; the bridge is live-query only, so cleared/focus-changed selections disappear from the next AX query.

8. Leave VoiceOver announcements and non-macOS bridges unchanged. This slice adds the macOS selected-text attribute without changing `action_accessibility_contents` or adding a platform fallback elsewhere.

## Testing and validation
Already verified:

- Behavior 1, 4, 6, 7, 8, 9, 10, 11, 13, and 14: Tom dogfood-passed the macOS flow on 2026-06-19/20 with `accessibilitySelectedText` only, including output, multi-line, rectangular, wrapped, alt-screen, Shift-drag in a mouse-reporting TUI, secret-in-selection, no/cleared selection, wide characters, reversed selection, and VoiceOver on/off checks. macOS Speak Selection itself is not available in CI.
- Behavior 2: `test_accessibility_data_prefers_input_selected_text` covers input-editor selected text and verifies input selection still wins when terminal output is also selected.
- Behavior 3, 5, 6, 7, 9, 10, and 14: existing `selection_to_string` and selection tests cover the terminal selection semantics reused by this bridge. They do not replace the macOS dogfood proof; they pin the shared selected-text source.
- Behavior 11 and 12: source inspection verifies the bridge queries focused-view accessibility data live and does not cache selected text; existing selection-clearing paths remain the only invalidation path.
- Build proof from the feature brief: clean `cargo check` for the Rust slice, with `host_view.m` covered by the `-c oss` bundle build rather than cargo.

Owed / follow-up validation:

1. If this goes upstream as an implementation PR, include manual testing proof in the PR body per `CONTRIBUTING.md`; for this interaction, written evidence plus a narrated screen recording is stronger than a screenshot.
2. Keep the focused unit test for input precedence. Add FFI helper coverage if `#warp-36` introduces a shared string-return helper.
3. Re-test on older supported macOS versions only if a regression is reported; the known spike proof is macOS 26.4.1.

## Parallelization
Not proposed. The feature is already implemented, and the remaining work is a small, tightly coupled documentation/spec pass. Parallel agents would mainly create file-ownership risk in this shared checkout.

## Risks and mitigations
- **macOS AX is not CI-verifiable here:** mitigated by Tom's real-device dogfood pass and by keeping the bridge narrow over existing selected-text code.
- **Transcript rebuild cost:** `warp_get_accessibility_selected_text` currently obtains full accessibility data, which rebuilds the transcript just to read `selected_text`. Mitigate in `#warp-36` with a content-free selected-text accessor and shared FFI string helper.
- **Selection policy drift:** mitigated by using `selected_text_from_input(ctx).or_else(selection_to_string(...))`, matching the focused-pane selected-text policy and pinning input precedence in `test_accessibility_data_prefers_input_selected_text`.
