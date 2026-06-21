# Tech Spec: Skills hot-reload across home and project scopes

As-built status: implemented in `warp-rbf`, dogfood passed for home symlink, direct home, and project scopes. Code references are pinned to `c43ca7761615070e21ee8c25d7687f04482735fe`; the original `#warp-01` feature slice was reconciled against `0705dc63299838abc441c162ead3214fb752a948`.

## Context

The behavior is specified in [product.md](product.md). The implementation spans the skills watcher/subscriber boundary and the manager cache that serves available skills.

- [`app/src/ai/skills/file_watchers/subscribers.rs:22-35 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/subscribers.rs#L22-L35) resolves the skills directory for a home provider from the original provider path, not the canonical watcher root.
- [`app/src/ai/skills/file_watchers/subscribers.rs:118-150 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/subscribers.rs#L118-L150) carries that original `provider_path` through `HomeSkillSubscriber::on_scan`.
- [`app/src/ai/skills/file_watchers/skill_watcher.rs:44-99 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher.rs#L44-L99) owns watcher state, including the provider-parent `home_provider_canonical_to_originals` map and the older per-skill `symlink_canonical_to_originals` map.
- [`app/src/ai/skills/file_watchers/skill_watcher.rs:610-735 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher.rs#L610-L735) translates canonical repository updates back to original provider paths before normal home-skill filtering runs.
- [`app/src/ai/skills/file_watchers/skill_watcher.rs:735-858 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher.rs#L735-L858) parses added/modified/moved home skill files and emits deleted paths after translation.
- [`app/src/ai/skills/file_watchers/skill_watcher.rs:954-1025 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher.rs#L954-L1025) preserves the existing per-skill symlink target route for already-loaded symlinked skill files.
- [`app/src/ai/skills/file_watchers/skill_watcher.rs:1038-1290 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher.rs#L1038-L1290) handles provider creation/deletion, direct Warp-managed paths, rollback cleanup, and retarget-safe provider watcher registration.
- [`app/src/ai/skills/file_watchers/skill_watcher.rs:250-430 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher.rs#L250-L430) refreshes project skills through repo metadata and guards asynchronous refreshes with generations.
- [`app/src/ai/skills/file_watchers/skill_watcher.rs:541-610 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher.rs#L541-L610) keeps failed local project repos hot-reloaded through a direct project watcher fallback.
- [`app/src/ai/skills/skill_manager.rs:98-187 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/skill_manager.rs#L98-L187) scopes skills to home, project, local, remote, and cloud contexts before deduplication.
- [`app/src/ai/skills/skill_manager.rs:586-656 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/skill_manager.rs#L586-L656) removes deleted skill paths from every index and prunes stale same-name local siblings.
- [`crates/repo_metadata/src/watcher.rs:681-713 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/crates/repo_metadata/src/watcher.rs#L681-L713) defines the `RepositoryUpdate` shape that translation must rebuild without dropping non-path fields.

## Proposed changes

This records the implementation that shipped.

1. Startup scan uses original provider paths. `HomeSkillSubscriber` stores the user-facing provider parent and calls `scan_skills_for_home_provider(provider_path, home_dir)`. A symlinked `~/.agents` therefore matches the `SKILL_PROVIDER_DEFINITIONS` candidate built from `dirs::home_dir()` instead of comparing against the canonical target and returning no skills.

2. Home-provider watcher registration records provider-parent canonical mappings. `watch_home_provider_path` canonicalizes the original provider path through `StandardizedPath::from_local_canonicalized`; if the canonical path differs, it inserts `canonical -> original` into `home_provider_canonical_to_originals`.

3. Incremental home-provider updates translate at the boundary. `handle_repository_update` short-circuits when no provider mapping is registered; otherwise it rebuilds the `RepositoryUpdate` with canonical paths translated to original paths, then lets the existing home-skill parsing and delete logic run.

4. Translation is deterministic and complete for the symlink cases this feature owns. It chooses the deepest matching canonical prefix, fans out shared canonical targets to every original provider, keeps the identity path when the canonical target is also a provider, and salvages mismatched move fan-out as independent adds and deletes. `commit_updated`, `index_lock_detected`, and `remote_ref_updated` pass through unchanged.

5. Provider lifecycle cleanup is ownership-aware. Provider deletion stops the stored subscriber and removes that original from every canonical mapping. Registration failure removes only the failed registration's mapping. Re-registering the same original path purges the old mapping before adding the new one, stops the superseded subscriber, and guards async rollback by the exact `(repo handle, subscriber id)` that failed.

6. The existing per-skill symlink target route stays separate. `symlink_canonical_to_originals` remains the file-level map for already-loaded symlinked skill files and `SymlinkTargetUpdate`; the new provider-parent map handles canonical events from provider directory watches. The route-unification refactor is intentionally deferred to `#warp-35`.

7. Project skills continue through RepoMetadata first, with local filesystem fallback only when metadata indexing fails. This slice does not add a project-provider canonical translation layer; it preserves the existing project live-update path and validates that create/rename/remove still work for active project skills.

8. SkillManager deletion now revisits same-name siblings. `handle_path_deleted` records names touched by a delete, `remove_skill_path_from_indexes` removes paths from `directory_skills`, `skills_by_path`, and `skills_by_name`, and `prune_stale_skill_paths_for_names` drops local same-name paths whose files no longer exist while leaving remote paths intact.

9. The feature is unconditional. There is no `FeatureFlag`, setting, command, visible UI, or migration. The only user-visible change is that live skill availability stops going stale.

## Testing and validation

1. Product behavior 1-2: `scan_skills_for_home_provider_finds_skills_under_original_path` and `scan_skills_for_home_provider_returns_none_for_unmatched_path` pin startup scanning from original provider paths. See [`subscribers.rs:176-224 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/subscribers.rs#L176-L224).

2. Product behavior 3 and 16: `test_handle_repository_update_single_skill_added` covers the unchanged direct-path add behavior, and `test_handle_repository_update_translates_canonical_paths_for_symlinked_provider` proves a canonical event under a registered symlinked provider emits `SkillsAdded` with the original path and no duplicate event. See [`skill_watcher_tests.rs:155-249 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher_tests.rs#L155-L249).

3. Product behavior 4 and 8: `test_handle_repository_update_fans_out_to_all_originals_for_shared_canonical` and `test_handle_repository_update_salvages_mismatched_moves_as_add_and_delete` cover shared-canonical fan-out and cross-boundary move salvage. See [`skill_watcher_tests.rs:937-1108 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher_tests.rs#L937-L1108).

4. Product behavior 9-10: `test_translate_canonical_picks_deepest_prefix_match` and `test_translate_keeps_identity_when_canonical_is_itself_a_provider` cover nested canonical mappings and identity fan-out. See [`skill_watcher_tests.rs:1109-1215 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher_tests.rs#L1109-L1215).

5. Product behavior 11-13: `test_watch_home_provider_path_reregistration_purges_stale_canonical_mapping` covers symlink retarget purge, superseded subscriber shutdown, and late rollback not tearing down the newer mapping. See [`skill_watcher_tests.rs:1344-1494 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher_tests.rs#L1344-L1494).

6. Product behavior 5: `handle_skills_deleted_prunes_stale_same_name_paths` covers stale same-name sibling pruning while preserving the live sibling. See [`skill_manager_tests.rs:1525-1595 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/skill_manager_tests.rs#L1525-L1595).

7. Product behavior 7: project-skill live behavior is covered by metadata refresh and fallback tests: stale refresh results are ignored, metadata refresh avoids unnecessary fallback watchers, indexed and symlinked project skill directories load, local fallback scans the filesystem, fallback updates reuse the repository update handler, added directories scan directly, and missing project skill paths emit deletes. See [`skill_watcher_tests.rs:298-639 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher_tests.rs#L298-L639) and [`skill_watcher_tests.rs:877-930 @ c43ca776`](https://github.com/rigelblu/warp-rbf/blob/c43ca7761615070e21ee8c25d7687f04482735fe/app/src/ai/skills/file_watchers/skill_watcher_tests.rs#L877-L930).

8. Product behavior 6 and 14-15: direct home provider and provider lifecycle coverage is partly route-level and partly dogfood-owned. The code paths are covered by `handle_warp_managed_paths_event`, `handle_home_files_changed`, and the generic repository update tests above; the final app-surface gate is Tom's dogfood pass recorded in the feature brief: create, rename, and remove under symlinked `~/.agents`, direct `~/.warp/skills`, and active project `<project>/.agents/skills` all hot-reloaded live with no restart.

9. Settled command evidence from the brief: `cargo test -p warp skill` passed with the 10 net-new tests plus existing `handle_repository_update` coverage; focused rollback regression `CARGO_TARGET_DIR=/private/tmp/warp-rbf-target cargo test -p warp test_watch_home_provider_path_reregistration_purges_stale_canonical_mapping -- --nocapture` passed 1/1 on 2026-06-25.

## Parallelization

No parallel implementation split is recommended for this as-built feature. The startup scan, provider translation, watcher lifecycle, and manager deletion changes share one watcher/manager contract; splitting them into separate worktrees would create coordination overhead around the same invariants. Parallel read-only review agents were useful for finding the rollback race, but the code and tests should land as one cohesive slice.

## Risks and mitigations

1. The two canonical-to-original maps can drift conceptually. Mitigation: keep their responsibilities explicit in comments and tests until `#warp-35` unifies the routes.

2. App-level regression coverage is still thinner than watcher-level coverage. Mitigation: keep Tom's real app dogfood as the visible-surface gate and add a future integration harness that can launch with temp home/project dirs, mutate skill files, and observe the active skill surface.

3. Filesystem existence checks during stale sibling pruning can block on slow local filesystems. Mitigation: the check is limited to same-name local siblings touched by a delete; remote paths are not filesystem-checked.

## Follow-ups

1. `#warp-35`: unify `SymlinkTargetUpdate` and `RepositoryUpdate` translation routes so a single canonical-to-original map serves provider-parent and skill-file symlink cases.
2. Add an integration-level active-skill harness for temp home, direct home, and temp project scopes so the app-visible dogfood scenarios can become agent-owned gates.
