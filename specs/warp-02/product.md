# macOS Speak Selection reads the selection

## Summary
On macOS, Speak Selection reads the active Warp text selection instead of starting at the top of the terminal pane. The selected-text behavior matches Warp's existing copy semantics for input text, terminal output, alt-screen selections, rectangular selections, wrapped lines, and obfuscated secrets.

## Problem
Before this fix, Warp exposed the terminal transcript through macOS accessibility but did not expose the active selected text. When a user selected text in Warp and pressed Option+Esc, macOS fell back to reading the text area's full accessibility value from the beginning, which made Warp inconsistent with other terminals and broke an accessibility workflow used for proof-listening selected text.

## Goals / Non-goals
Goals:
- Speak the active selected text in the focused Warp terminal/input surface.
- Keep Speak Selection aligned with copy semantics for ordering, wrapping, rectangular selections, alt-screen selections, and secret obfuscation.
- Avoid stale or cross-pane speech after the selection changes, clears, or focus moves.

Non-goals:
- No changes to copy, copy-on-select, AI-context selection, or VoiceOver announcement semantics beyond avoiding regressions.
- No non-macOS platform bridge in this slice.
- No new visual UI, settings surface, prompt, or fallback menu.
- No performance rewrite for the full accessibility transcript path; that remains a follow-up.

## Behavior
1. When the focused Warp terminal has a non-empty text selection and the user invokes macOS Speak Selection with Option+Esc, macOS speaks that selection, starting at its first selected character, instead of starting at the top of the pane.

2. Input-editor selection has precedence over terminal-output selection. If the focused terminal input/editor contains selected text, Speak Selection speaks that input selection even when terminal output also has an active text selection.

3. When there is no input-editor selection, Speak Selection uses the focused terminal output selection. A selection dragged bottom-up or right-to-left is spoken in document order, matching copy behavior.

4. Multi-line selections speak the selected range and stop at its end. The speech does not continue into unselected rows or earlier terminal content.

5. Word and line selections speak the expanded highlighted selection, not only the single mouse-down cell.

6. Rectangular selections preserve row boundaries and plain-text ordering in the same way Warp copy does.

7. Wrapped-line selections use copy's line-break semantics. Visual wraps do not introduce extra spoken line breaks unless copy would include them.

8. Alt-screen selections speak only the visible selected alt-screen text. In mouse-reporting TUIs, a normal drag belongs to the TUI; holding Shift while dragging lets Warp own the terminal text selection, and Speak Selection then reads that selection.

9. Obfuscated secrets remain obfuscated through the accessibility selected-text path. Speak Selection must not reveal hidden secret values that copy would hide.

10. With no active text selection, a block-selection-only state, or an empty selection, Warp does not populate selected text for macOS. It must not reuse a stale previous selection.

11. Clearing or invalidating a selection through clear, command execution, alt-screen transition, resize, or focus-changing selection cleanup is reflected in the next Speak Selection query. The native bridge queries live state and does not cache selected text.

12. Only the focused surface contributes selected text. A selection in an unfocused pane does not override the focused terminal or non-terminal surface.

13. Existing VoiceOver focus and selection announcements continue to work. Speak Selection support must not introduce duplicate VoiceOver speech or remove the existing block-text announcement path.

14. Wide characters, emoji, combining marks, and non-Latin text are spoken as the selected glyphs. The selected-text path must not duplicate spacer cells, truncate wide characters, or split combining sequences differently from copy.
