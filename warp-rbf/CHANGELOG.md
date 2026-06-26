---
title: Changelog
---

# 🔵⋯ [Unreleased]

---

# 🔵⋯ v0.8.0 (2026-06-30)
## 🟠⋯ Added
- 2026-06-29 - feat (user need) | know exactly which Warp build and RBF feature set is running before trusting a dogfood result (#warp-47)

---

# 🔵⋯ v0.7.0 (2026-06-23)
## 🟠⋯ Added
- name a window from Warp's command surface | use `/name-window <name>` to give the current window a persistent name that shows in macOS window surfaces and survives restart; `/name-window --clear` returns to the active-tab title (#warp-06)

# 🔵⋯ v0.6.0 (2026-06-21)
## 🟠⋯ Added
- colored vertical tabs read clearly, and the active one is unmistakable | a colored tab shows a full-strength color stripe on its leading edge with a faint matching row tint; the active tab fills its row with a strong tint of its color — the one vivid block, with white text on saturated colors (#warp-32)
- colored tabs in the horizontal tab bar match — the active one is unmistakable too | a colored tab in the top bar shows a full-strength color bar on its bottom edge with a faint matching tint; the active tab fills with a strong tint of its color — the one vivid block, with white text on saturated colors — and its bottom bar flips to a contrasting edge so it stays a clean divider (#warp-32)
- renaming a colored tab stays legible | while you rename a colored tab, its row drops the color fill so the inline rename editor reads clearly against the theme's default text, then the color returns when you finish (#warp-32)
- name the color of a tab on hover | hovering a tab's color swatch shows the nearest named color, so you can tell which color a tab is set to (#warp-32)

# 🔵⋯ v0.5.0 (2026-06-21)
## 🟠⋯ Added
- collapse or expand a pane into a thin edge rail | tuck a pane you're not using to a thin rail on its edge to reclaim its space without closing it — `⌥⇧`+`h`/`j`/`k`/`l` rails the neighbor (or a whole bordering column/row) in place, repeat presses sweep further, `⌥⇧E` expands the focused pane to fill the tab, and restore with the opposite key or a click (#warp-03)

# 🔵⋯ v0.4.0 (2026-06-21)
## 🟠⋯ Fixed
- macOS Speak Selection reads the selection, not the whole pane | with text selected, Speak Selection (`⌥Esc`) reads just the selection instead of starting from the top of the pane (#warp-02)

# 🔵⋯ v0.3.0 (2026-06-21)
## 🟠⋯ Added
- skills hot-reload across home and project scopes | edited, added, or removed skills in home-level providers or the active project reload live instead of going stale until you restart (#warp-01)

# 🔵⋯ v0.2.0 (2026-06-21)
## 🟠⋯ Improved
- tab groups, vertical tabs, and per-directory tab colors in WarpOss | the OSS build ships the tab features that are preview-gated in upstream Warp (#warp-38)

# 🔵⋯ v0.1.0 (2026-06-21)
## 🟠⋯ Fixed
- OSS dogfood build stops re-prompting for data access on every launch | local WarpOss builds self-sign with a stable identity, so a granted Full Disk Access sticks across rebuilds (#warp-37)
