---
title: Changelog
---

# 🔵⋯ [Unreleased]

---

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
