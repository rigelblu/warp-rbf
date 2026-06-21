---
title: "Warp RBF"
---

This directory holds this flavour's docs, scripts, and version metadata.

# 🔵⋯ Use
This is for my personal use and shared publicly for those curious.
I'm not accepting any issues, contributions, etc.

# 🔵⋯ Context
This is my ~~fork~~ flavour of [Warp](https://github.com/warpdotdev/Warp), similar to [Zed RBF](https://github.com/rigelblu/zed-rbf)
> The idea started here [My own Zed flavour with a clone of zed's codebase as the base. Maybe the future isn't extensions or plugins](https://x.com/thosiawa/status/2016222485608272202?s=20) when I wanted Bear Markdown features (i.e. coloured highlights) in Zed. I started out wanting to make it an extension, but many features required changes to Zed's core. I use a script to have agents sync from Zed's upstream and let them handle the conflicts every week.

> It's not vibe coded, but I also haven't looked at any code (my software development, architecture, Rust skills aren't there yet). I pay very close attention to the other parts though, the "why" and the "what". Claude/Codex generated and reviewed all the code through my extensive skill system, based on everything I've learned and how I think from the last 15 years working on products. I rebuilt it from my product ship plan and feature design briefs to create a clean commit history and make it easier to consume if others are curious. Manually verified every feature myself too.

# 🔵⋯ Features
**Data-access grants**
— WarpOOSS self-signs with a stable identity, so a macOS data-access grant survives rebuilds instead of re-prompting 7–20× every launch.

**Enable vertical tabs an groups**
— tab groups (macOS), the vertical tab layout, and directory-colored tabs.

**Live skill hot-reload**
— add, rename, or remove skills in your home or project skills dirs and they reload without restarting Warp.

**Speak Selection reads the selection** (macOS)
— Option+Esc on selected terminal text reads what you selected, not from the top of the pane.

## 🟠⋯ Settings, Themes, And Data
Copy your regular warp configs
```
cp -a $HOME/.warp $HOME/.warp-oss
```

---

# 🔵⋯ Prerequisites
<TODO>

# 🔵⋯ Getting Started
<TODO>
## 🟠⋯ Install Toolchain
```sh
xcodebuild -downloadComponent MetalToolchain
```

## 🟠⋯ Build And Run
Compiles and launches WarpOss from source — nothing is installed.
```sh
./script/run                        # dev profile — fast build, UNoptimized; for iterating
./script/run --profile release-lto  # production-grade (opt-level 3 + thin LTO); matches the installed app
```
- `./script/run` defaults to the **dev** profile — quick rebuilds but debug-speed; don't daily-drive it.
- Logs are a **runtime** knob, not a build — every profile keeps its log statements.
  - Raise verbosity with `RUST_LOG` (output goes to `warp-oss.log`).
  - e.g. `RUST_LOG=debug ./script/run --profile release-lto`, or `RUST_LOG=warp=debug` to scope to Warp's crates.

## 🟠⋯ Install as Local App
Builds an optimized, self-signed `WarpOss.app`. Run from the repository root:
```sh
./script/bundle -c oss
```
- The `oss` channel builds the **`release-lto`** profile (opt-level 3 + thin LTO, assertions off) — production performance.
  - This, not `./script/run`, is the build to daily-drive.
- Self-signs with your local Apple Development cert (a stable identity), so a granted **Full Disk Access** sticks across rebuilds.
- Want production speed **plus** runtime assertions (catch invariant bugs)? Build `-c dev` instead — it uses `release-lto-debug_assertions`.
- Cargo profiles live in `Cargo.toml`; the bundle maps each channel to one (`oss`/`preview`/`stable` → `release-lto`).

# 🔵⋯ FAQ
<TODO>

# 🔵⋯ Troubleshooting
## 🟠⋯ "cannot execute tool 'metal' due to missing Metal Toolchain" Persists After Installing It
Run these commands when build logs include `error: cannot execute tool 'metal' due to missing Metal Toolchain; use: xcodebuild -downloadComponent MetalToolchain`
```sh
xcodebuild -downloadComponent MetalToolchain
xcrun -k
```
