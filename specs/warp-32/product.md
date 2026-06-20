# Colored Tabs Read Clearly

## Summary

Colored tabs should be readable as colors when inactive and unmistakable as the active tab when selected. Warp now splits color identity from active state: inactive colored tabs keep a full-strength color strip/accent, while the active colored tab becomes the single vivid filled tab with readable nonsemantic content.

## Problem

The old semi-transparent color wash made tab colors backdrop-dependent. On themes and background images with close hues, inactive colors looked washed out and the active colored tab was hard to distinguish from its neighbors.

## Goals / Non-goals

Goals:

- Make colored tabs readable in both the vertical panel and horizontal tab bar.
- Make the active colored tab visually dominant without changing the tab color model.
- Keep inline rename legible while editing a colored tab name.

Non-goals:

- Add custom colors, per-color tuning, persistence changes, or a new color model.
- Change non-colored tab styling.
- Keep the colored fill visible during inline rename.

## Figma

Figma: none provided.

## Behavior

1. When a tab has a color and is inactive, Warp shows the color as an opaque identity mark rather than relying on a washed-out fill. In the vertical panel this is a leading strip; in the horizontal bar this is a bottom accent.

2. Inactive colored tabs also keep only a faint same-color tint behind the identity mark: 14% opacity at rest and 24% opacity on hover. The tint is flavor, not the primary color signal.

3. When a colored tab becomes the active tab, that tab inverts: the tab or row fills with a strong 90% tint of its own color and becomes the one vivid block in its tab surface.

4. The active colored tab's strip or bottom accent flips to a contrasting edge derived from the active fill, so the identity mark remains visible instead of disappearing into the same-color fill.

5. The active colored tab keeps slight background-image composition because the fill remains 90% opaque rather than fully opaque.

6. Non-colored tabs do not change. Active non-colored tabs continue to use the existing neutral selected lift, and inactive non-colored tabs continue to use the existing hover/rest styling.

7. The vertical panel and horizontal tab bar both implement the same color-state recipe. A user should not see a colored tab read clearly in one layout and wash out or lose active-state signal in the other.

8. The active colored tab's title text remains readable on its fill. Dark and saturated mid-luminance fills use white-biased text; genuinely light fills use the normal dark text picked for that fill.

9. In the horizontal tab bar, the active colored foreground treatment applies to nonsemantic content sitting on the active fill: the tab title, compact fallback icon, pinned indicator, unsaved-changes dot, maximize icon, statusless Oz indicator, and ambient-agent cloud icon.

10. Horizontal semantic or status-bearing indicators keep their semantic colors unless a later dogfeel pass proves they fail: synced input, error, shared session, shell indicator, and agent conversation-status icons.

11. In the vertical panel, active colored row text and metadata use foreground colors picked from the active fill in expanded, summary, and compact modes.

12. In the vertical panel, row-derived nonsemantic icon fills use the same readable foreground treatment when they sit on the active colored fill. Brand, Drive-object, and status icons keep their own colors.

13. When a user renames a colored tab in the horizontal tab bar, Warp temporarily treats that tab as uncolored before deriving the tab background, accent, close-button hover fill, and foreground. The inline editor appears on the neutral selected lift for the duration of editing.

14. When a user renames a colored tab or pane in the vertical panel, Warp temporarily treats the corresponding row as uncolored before deriving row paint and row content colors. Expanded, summary, and compact vertical rows all use the neutral selected lift while the inline editor is active.

15. After rename ends, the colored tab returns to the normal colored-tab recipe for its current state: faint tint plus full-strength strip/accent when inactive, or 90% active fill plus contrasting edge when active.

16. Multi-selection and drag states keep their existing neutral selected/drag styling unless the row is the focused active colored tab. The colored-tab recipe should not make multi-selected inactive rows look active.

17. The feature is always on and does not introduce a user-visible preference, feature flag, persistence change, or migration.

18. The design intentionally accepts that near-twin hues can still require names or custom colors to distinguish semantically. This feature makes the color signal stronger; it does not solve the color-model problem deferred to follow-up work.

## Open questions

- Tom dogfeel has passed the base vertical and horizontal inversion. The 2026-06-25 review fixes for horizontal rename, horizontal compact/statusless glyphs, and vertical rename content are agent-verified but still need Tom dogfeel confirmation.
