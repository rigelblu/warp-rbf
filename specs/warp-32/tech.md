# Colored Tabs Read Clearly Tech Spec

Status: implemented as-built at `c43ca7761615070e21ee8c25d7687f04482735fe`. Base vertical and horizontal inversion has Tom dogfeel pass; the 2026-06-25 review fixes are agent-verified and still need Tom dogfeel. Code references below are pinned to that local fork commit and use `rigelblu/warp-rbf` links because the commit is not present on an `origin/*` remote-tracking ref in this checkout.

## Context

`product.md` defines the user-visible behavior. The shipped implementation is a pure render change: it does not add tab-color state, persistence, settings schema, or migration work.

Relevant code at `c43ca7761615070e21ee8c25d7687f04482735fe`:

- [`app/src/tab.rs:75-129 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/tab.rs#L75-L129) defines the horizontal recipe constants, extracts the solid tab color, and picks active-colored title text.
- [`app/src/tab.rs:1100-1115 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/tab.rs#L1100-L1115) centralizes horizontal rename neutralization and effective foreground selection.
- [`app/src/tab.rs:1239-1505 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/tab.rs#L1239-L1505) applies the effective foreground to title, pin, unsaved dot, maximize icon, statusless Oz, and ambient-agent icon while leaving semantic indicators on their existing colors.
- [`app/src/tab.rs:1547-1830 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/tab.rs#L1547-L1830) derives the horizontal active fill, idle/hover tint, contrasting bottom accent, close-button hover fill, and grouped-member accent.
- [`app/src/workspace/view/vertical_tabs.rs:121-459 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view/vertical_tabs.rs#L121-L459) defines the vertical constants, row background, content background, and active-colored text colors.
- [`app/src/workspace/view/vertical_tabs.rs:464-581 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view/vertical_tabs.rs#L464-L581) renders the vertical leading strip and derives the pinned indicator color from the effective row background.
- [`app/src/workspace/view/vertical_tabs.rs:2196-2284 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view/vertical_tabs.rs#L2196-L2284) and [`app/src/workspace/view/vertical_tabs.rs:2316-2357 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view/vertical_tabs.rs#L2316-L2357) pass tab/pane rename state into vertical summary, expanded, and compact row props.
- [`app/src/workspace/view/vertical_tabs.rs:3718-3823 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view/vertical_tabs.rs#L3718-L3823) makes `PaneProps::effective_pane_color()` the single source of truth for dropping color during rename.
- [`app/src/workspace/view/vertical_tabs.rs:3324-3431 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view/vertical_tabs.rs#L3324-L3431), [`app/src/workspace/view/vertical_tabs.rs:4537-4741 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view/vertical_tabs.rs#L4537-L4741), and [`app/src/workspace/view/vertical_tabs.rs:7042-7250 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/workspace/view/vertical_tabs.rs#L7042-L7250) apply vertical text/icon colors through expanded, summary, and compact rows.

## Proposed changes

The implementation is already present; keep it as the shipped design rather than adding a new abstraction.

1. Preserve the two-render-path shape. Horizontal tabs and vertical tabs do not share renderer code today, so both paths intentionally carry parallel constants and comments for the same recipe. A shared helper would create churn across UI layers without removing much behavior risk.

2. Keep horizontal color derivation centered on `TabElement::effective_tab_background()` and `TabElement::effective_tab_foreground()`. Any future horizontal child that renders nonsemantic content on the active colored fill should ask for the effective foreground instead of manually picking `styles.default.font_color`.

3. Keep vertical color derivation centered on `PaneProps::effective_pane_color()` and `pane_row_text_colors()`. Row paint, row content, expanded rows, summary rows, and compact rows should all derive from the same effective pane color so rename cannot neutralize only part of the row.

4. Keep semantic colors scoped out of the generic contrast override. Status, brand, shell, Drive-object, and conversation-state glyphs communicate meaning through color; they should change only with a specific dogfeel failure or a later semantic-color design.

5. Do not introduce data-model, settings, or migration work in `warp-32`. Custom colors and per-color tuning belong to the follow-up color-model work, not this render-only slice.

## Testing and validation

Existing proof from the brief:

- Product invariants 1-8, 11-12, and 15: Tom dogfeel passed for the base inversion in vertical and horizontal layouts across multiple colored tabs, near-twin hues, a dark fill, a light fill, rose-pine-pink-city-dawn with a background image, rose-pine-dawn, and a dark theme.
- Product invariants 13-14: vertical rename had Tom dogfeel pass in the earlier rename slice; horizontal rename and vertical content cleanup are agent-verified at the 2026-06-25 review-fix snapshot.
- Product invariants 6, 16, and 17: code inspection confirms the change is render-only and falls back to existing neutral styling for uncolored, multi-selection, drag, and no-color paths.
- Agent verification recorded in the brief: `git diff --check -- app/src/tab.rs app/src/workspace/view/vertical_tabs.rs`; `cargo check -p warp` passed with existing unrelated warnings.

Recommended before promoting the review fixes beyond agent verification:

1. Cover product invariants 9-10 manually in the horizontal bar: make the active tab colored, shrink it below compact threshold, pin it, add unsaved code changes, maximize a pane, and compare statusless Oz/ambient-agent indicators against semantic Synced/Error/Shared/shell/status icons.

2. Cover product invariants 13 and 15 manually in the horizontal bar: rename an active colored tab, confirm the fill drops to neutral while editing, confirm the inline text remains legible, then commit/cancel and confirm the color returns.

3. Cover product invariants 11-14 manually in the vertical panel across Expanded, Summary, and Compact modes: rename a colored active tab and a colored pane where available; confirm row paint and title/metadata colors all neutralize together during edit.

4. Re-run `git diff --check -- app/src/tab.rs app/src/workspace/view/vertical_tabs.rs specs/warp-32/product.md specs/warp-32/tech.md` after any follow-up edit.

5. Consider a future render-focused regression guard around the constants and effective-color entry points. There is no existing automated visual coverage that proves the exact opacity or luminance behavior; manual dogfeel remains the real gate for this slice.

## Risks and mitigations

- Two render paths can drift. Mitigation: keep the parallel constants and explanatory comments in both files, and require future tab-color changes to inspect both `tab.rs` and `vertical_tabs.rs`.
- Semantic glyphs may be less readable on some active fills because they intentionally keep their meaning colors. Mitigation: defer until dogfeel identifies a real failing glyph, then solve that semantic case directly.
- The uniform 90% fill and 0.5 luminance threshold are perceptual choices, not model-aware color science. Mitigation: keep custom colors and per-color tuning in follow-up color-model work.

## Follow-ups

- Product ship plan reconciliation: the plan entry still says rename neutralizes fill vertical-only and says `cargo check -p warp` was clean with 0 warnings; the brief is the current truth for this spec.
- Tom dogfeel for the 2026-06-25 review fixes remains open.
