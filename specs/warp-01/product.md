# Product Spec: Skills hot-reload across home and project scopes

## Summary

Warp keeps file-backed skills current while the app is running. Creating, editing, renaming, moving, or deleting skills in home-level providers and active project skill directories updates the available skill set live, including when a home provider such as `~/.agents` is a symlink into a dotfiles checkout.

## Problem

Skill authors expect edits to `SKILL.md` files to appear in Warp without a restart. In the broken symlinked-home-provider case, the filesystem watcher received canonical paths from the symlink target while the skill filters compared against the original home-provider path, so startup scans and incremental events silently missed the skills. The failure was invisible: a user edited a skill, Warp kept showing stale behavior, and there was no clear signal that a restart was needed.

## Goals / Non-goals

Goals:

1. Home-level skills hot-reload live for both direct providers and providers whose parent directory is symlinked.
2. Active project-level skills hot-reload live without regressing their existing project-scoped behavior.
3. Skill removal and rename flows remove stale entries instead of leaving old skill names available.
4. The fix is always on; users do not need a feature flag, setting, command, or manual refresh.

Non-goals:

1. This feature does not change the skill file format, skill invocation UI, or provider priority rules.
2. This feature does not add a new visible error surface for invalid skill files; invalid or unparsable skills continue not to appear as available skills.
3. This feature does not unify the home-provider and per-skill symlink watcher routes. That cleanup is tracked separately as `#warp-35`.

## Behavior

1. On launch, Warp scans home-level skill providers and available project skill files, then makes valid skills available without requiring a user action beyond starting the app.

2. If a home provider parent path is a symlink, such as `~/.agents -> ~/dotfiles/agents`, Warp treats the original user-facing provider path as the provider identity. Skills under the symlinked provider load at startup the same way they would under a direct provider.

3. While Warp is running, creating or editing a valid `SKILL.md` under a symlinked home provider makes that skill available live, no restart, tab close, app refresh, or manual reload required.

4. While Warp is running, renaming or moving a valid skill under a symlinked home provider removes the old skill path and makes the new skill path available live. Cross-boundary moves that cannot be represented as a one-to-one move are still surfaced as an add plus a delete rather than being dropped.

5. While Warp is running, deleting a skill file or skill directory under a symlinked home provider removes that skill live. If another same-name indexed path points at a local file that is also gone, Warp removes that stale sibling too; a same-name sibling whose local file still exists remains available.

6. Direct home providers, including Warp-managed skill directories such as `~/.warp/skills`, keep the same live create, edit, rename, move, and delete behavior as symlinked home providers.

7. Active project-level skills under `<project>/.agents/skills` hot-reload live for that project. Creating, editing, renaming, moving, or deleting a valid project skill updates availability for matching project working directories without turning the skill into a global home skill or leaking it into unrelated projects.

8. If multiple home provider paths resolve to the same canonical directory, a single filesystem event from that canonical directory updates each original provider path that depends on it. No provider alias should starve another provider of events.

9. If the canonical target of one symlinked provider is itself also a registered provider, Warp keeps both the identity path and the alias path in the update fan-out. The real provider and the alias provider both stay current.

10. When canonical provider mappings are nested, Warp translates events using the deepest matching canonical prefix. Nested symlink targets must not translate through a shallower provider by accident.

11. If a provider symlink is retargeted while Warp is running, Warp drops the old canonical mapping for that original provider path and registers the new target. Later events from the old target must not translate into phantom skills under the original provider.

12. A late failure from a superseded watcher registration must not roll back a newer, live watcher registration for the same original provider path. The newer mapping and subscriber remain authoritative.

13. If watcher registration fails for a home provider, Warp removes any canonical mapping inserted for that failed registration. The map of canonical-to-original provider paths reflects only committed watches.

14. If a home provider directory is deleted, Warp stops that provider watcher, removes its canonical mapping, and removes the provider's skills from the available skill set. If the provider directory is created or moved into place later, Warp starts watching it and loads valid skills from it.

15. Non-skill filesystem changes under watched directories do not create visible skills. Skill availability changes only when a valid skill file or skill directory is added, changed, moved, or deleted.

16. In non-symlinked setups, the hot-reload behavior remains the existing direct-path behavior. The symlink translation path must not add duplicate events or change visible behavior when no canonical provider mapping is registered.
