# Name a window

## Summary
Warp users can give the current window a persistent custom name with `/name-window <name>`. While set, the name overrides the active-tab-derived window title, appears in macOS window-selection surfaces, and survives restart until the user clears it with `/name-window --clear`.

## Problem
Warp window titles are currently derived from the active tab, so a window's identity changes when the user switches tabs. In multi-window setups, macOS title bars, the Window menu, Mission Control, and the app switcher become less useful because the window picker reflects the active tab's current title instead of the window's purpose.

## Goals / Non-goals
Goals:
- Let a user name the current Warp window by purpose.
- Keep the named window title stable across tab switches and restarts.
- Use Warp's native slash-command surface instead of copying browser menu/dialog chrome.
- Keep the macOS slice honest: macOS surfaces are verified here; Windows and Linux rollout remains separate.

Non-goals:
- No macOS Window-menu command, Chrome-style dialog, titlebar editor, settings UI, or new prompt.
- No automatic naming, project-name suggestions, templating, shell-variable expansion, or per-tab window-name overrides.
- No claim that Windows or Linux OS surfaces are shipped by this slice.

## Figma
Figma: none provided.

## Behavior
1. When no custom window name is set, Warp behaves exactly as it did before this feature: the OS window title is derived from the active tab title and updates when the active tab changes.

2. `/name-window` is a Warp slash command. When Warp recognizes and executes it, the command is handled by Warp's command execution path, not by the shell, a shell alias, injected stdin, or a PTY-side escape sequence. If the user dismisses slash-command handling and leaves literal text in the terminal input, normal terminal submission behavior applies; that cancellation path is not a successful `/name-window` invocation.

3. `/name-window <name>` sets or replaces the current window's custom name. Warp treats the full argument after the command as the name, trims leading and trailing whitespace, preserves internal spaces, and rejects all-whitespace arguments once the argument has started.

4. After a custom name is set, the OS window title updates immediately to that name. On macOS, the title bar, Window menu, Mission Control, and app switcher show the custom name because they read the OS window title.

5. While a custom name is set, switching tabs does not change the OS window title. The custom name fully overrides the active-tab-derived title until it is cleared.

6. `/name-window --clear` removes the custom name for the current window and immediately restores active-tab-derived title behavior. Running `--clear` when no custom name is set is a quiet no-op.

7. Clearing is explicit. Exact bare `/name-window` stays in the slash UI's required-argument hint flow unless the user dismisses the menu; `/name-window ` and whitespace-only arguments show usage feedback and do not clear or overwrite the current name. Only an exact trimmed argument of `--clear` clears; other text, including `--clear later`, is treated as a normal window name.

8. Very long custom names are truncated through the same window-title truncation rule Warp already uses for active-tab-derived titles.

9. Window names are per-window. Naming one window does not affect other open windows, new windows, detached-tab windows, or restored windows that did not persist that name.

10. A custom window name persists through app-state save and session restore. After restart, a restored window with a saved custom name uses that name before the first user-visible title update that matters; a window whose name was cleared restores unset.

11. The slash command clears the invoking input after successful set or clear execution. The literal `/name-window ...` text must not appear as a shell command or be delivered to a running terminal/TUI when Warp recognizes and executes it as this command.

12. The command uses the existing slash-command discovery and feedback model. No new focus trap, modal semantics, pointer-only path, or visual component is introduced.

13. If the existing slash-command surface is unavailable because another terminal program owns raw input, this slice does not add a fallback macOS menu/dialog. The requirement is that recognized `/name-window` commands are consumed by Warp; unavailable slash-command contexts are a limitation to document, not a reason to add another product surface.
