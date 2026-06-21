use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use ai::skills::{ParsedSkill, SkillProvider, SkillScope};
use repo_metadata::entry::{DirectoryEntry, Entry, FileMetadata};
use repo_metadata::file_tree_store::FileTreeState;
use repo_metadata::repositories::DetectedRepositories;
use repo_metadata::repository::{Repository, RepositorySubscriber};
use repo_metadata::{
    DirectoryWatcher, RepoMetadataModel, RepositoryIdentifier, RepositoryUpdate,
    StandingQueryContent, StandingQueryResults, StandingQueryResultsDelta, TargetFile,
};
use tempfile::TempDir;
use warp_util::host_id::HostId;
use warp_util::local_or_remote_path::LocalOrRemotePath;
use warp_util::remote_path::RemotePath;
use warp_util::standardized_path::StandardizedPath;
use warpui::{App, SingletonEntity};

use super::super::subscribers::SkillRepositoryMessage;
use super::{parse_project_skill_contents, SkillWatcher};
use crate::ai::skills::skill_manager::SkillWatcherEvent;

/// A no-op subscriber used by the symlinked-provider tests that need a *real*
/// directory watch registered (so `watcher_count`/translation reflect a live
/// watcher) without caring about the scan/update callbacks themselves.
struct NoopRepositorySubscriber;

impl RepositorySubscriber for NoopRepositorySubscriber {
    fn on_scan(
        &mut self,
        _repository: &Repository,
        _ctx: &mut warpui::ModelContext<Repository>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        Box::pin(async {})
    }

    fn on_files_updated(
        &mut self,
        _repository: &Repository,
        _update: &RepositoryUpdate,
        _ctx: &mut warpui::ModelContext<Repository>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        Box::pin(async {})
    }
}

/// Helper function for creating a single skill file
fn create_skill_file(dir: &TempDir, name: &str, description: &str, content: &str) -> ParsedSkill {
    create_skill_file_in_directory(dir.path(), name, description, content)
}

fn create_skill_file_in_directory(
    parent_dir: &std::path::Path,
    name: &str,
    description: &str,
    content: &str,
) -> ParsedSkill {
    let skill_content = format!(
        r#"---
name: {}
description: {}
---
{}
"#,
        name, description, content
    );
    let skills_path = parent_dir.join(".agents").join("skills");
    let skill_dir_path = skills_path.join(name);
    let skill_file_path = skill_dir_path.join("SKILL.md");

    fs::create_dir_all(&skill_dir_path).unwrap();
    fs::write(&skill_file_path, skill_content.clone()).unwrap();
    let line_range_start = skill_content.clone().lines().count() - content.lines().count() + 1;
    let line_range_end = skill_content.clone().lines().count() + 1;
    ParsedSkill {
        path: LocalOrRemotePath::Local(skill_file_path),
        name: name.to_string(),
        description: description.to_string(),
        content: skill_content.clone(),
        line_range: Some(line_range_start..line_range_end),
        provider: SkillProvider::Agents,
        scope: SkillScope::Project,
    }
}

fn skill_local_path(skill: &ParsedSkill) -> PathBuf {
    skill.path.to_local_path().unwrap().to_path_buf()
}
fn remote_skill_path(host_id: &HostId, name: &str) -> LocalOrRemotePath {
    LocalOrRemotePath::Remote(RemotePath::new(
        host_id.clone(),
        StandardizedPath::try_new(format!("/repo/.agents/skills/{name}/SKILL.md").as_str())
            .unwrap(),
    ))
}

fn remote_skill_content(name: &str, description: &str, body: &str) -> String {
    format!(
        r#"---
name: {name}
description: {description}
---
{body}
"#
    )
}

#[test]
fn parse_project_skill_contents_preserves_remote_paths() {
    let host = HostId::new("test-host".to_string());
    let first_path = remote_skill_path(&host, "first");
    let second_path = remote_skill_path(&host, "second");
    let first_content = remote_skill_content("first", "First skill", "First body");
    let second_content = remote_skill_content("second", "Second skill", "Second body");

    let skills = parse_project_skill_contents(vec![
        (first_path.clone(), first_content.clone()),
        (second_path.clone(), second_content.clone()),
    ]);

    assert_eq!(skills.len(), 2);
    assert_eq!(skills[0].path, first_path);
    assert_eq!(skills[0].name, "first");
    assert_eq!(skills[0].content, first_content);
    assert_eq!(skills[0].provider, SkillProvider::Agents);
    assert_eq!(skills[1].path, second_path);
    assert_eq!(skills[1].name, "second");
    assert_eq!(skills[1].content, second_content);
}

#[test]
fn parse_project_skill_contents_classifies_foreign_encoded_provider_path() {
    let path = LocalOrRemotePath::Remote(RemotePath::new(
        HostId::new("test-host".to_string()),
        StandardizedPath::try_new(r"C:\repo\.codex\skills\windows-skill\SKILL.md").unwrap(),
    ));
    let content = remote_skill_content("windows-skill", "Windows skill", "Windows body");

    let skills = parse_project_skill_contents(vec![(path.clone(), content)]);

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].path, path);
    assert_eq!(skills[0].provider, SkillProvider::Codex);
}

// ============================================================================
// Tests for handle_repository_update
// ============================================================================

#[test]
fn test_handle_repository_update_single_skill_added() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill = create_skill_file(&temp_dir, "test", "Test skill", "Test content");

        let update = RepositoryUpdate {
            added: HashSet::from([TargetFile::new(skill_local_path(&skill), false)]),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        let event = rx.recv().await.unwrap();
        assert_eq!(
            event,
            SkillWatcherEvent::SkillsAdded {
                skills: vec![skill]
            }
        );
    });
}

/// warp-01 load-bearing regression (warpdotdev/warp#8897, upstream PR #9463):
/// when the provider parent (e.g. `~/.agents`) is itself a symlink, file events
/// fire under the canonical (resolved) path. Without translation, the downstream
/// filter — which compares against un-canonicalized `home_skills_path()` — would
/// silently drop them. Proves a canonical event under a registered symlinked
/// provider emits `SkillsAdded` with the *original* path.
#[test]
fn test_handle_repository_update_translates_canonical_paths_for_symlinked_provider() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        // The actual file lives at the *original* (un-canonicalized) location.
        let skill = create_skill_file(&temp_dir, "test", "Test skill", "Test content");
        let original_provider = temp_dir.path().join(".agents");
        let canonical_provider = temp_dir.path().join("dotfiles-agents");

        // Populate the canonical→originals map as `watch_home_provider_path` would
        // have done after `dunce::canonicalize` resolved the symlink at registration.
        skill_watcher_handle.update(&mut app, |watcher, _ctx| {
            watcher
                .home_provider_canonical_to_originals
                .entry(canonical_provider.clone())
                .or_default()
                .insert(original_provider);
        });

        // Event arrives with the canonical path (what FSEvents would emit when the
        // watch was registered on the symlink target).
        let canonical_skill_path = canonical_provider
            .join("skills")
            .join("test")
            .join("SKILL.md");
        let update = RepositoryUpdate {
            added: HashSet::from([TargetFile::new(canonical_skill_path, false)]),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        // The dispatched skill has the *original* path — translation worked, the
        // filter recognized it, and parse_skill read from the symlink-side location.
        let event = rx.recv().await.unwrap();
        assert_eq!(
            event,
            SkillWatcherEvent::SkillsAdded {
                skills: vec![skill]
            }
        );
        // Pin event cardinality: exactly one event, no duplicates from translation.
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn test_removing_remote_project_repo_deletes_shared_cached_skill_paths() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let host = HostId::new("test-host".to_string());
        let repo_id = RepositoryIdentifier::Remote(RemotePath::new(
            host.clone(),
            StandardizedPath::try_new("/repo").unwrap(),
        ));
        let first_path = remote_skill_path(&host, "first");
        let second_path = remote_skill_path(&host, "second");

        skill_watcher_handle.update(&mut app, |watcher, _| {
            watcher.project_skill_files_by_repo.insert(
                repo_id.clone(),
                HashSet::from([first_path.clone(), second_path.clone()]),
            );
            watcher.remove_project_skills_for_repo(&repo_id);
        });

        let SkillWatcherEvent::SkillsDeleted { mut paths } = rx.recv().await.unwrap() else {
            panic!("Expected SkillsDeleted event");
        };
        paths.sort_by_key(LocalOrRemotePath::display_path);
        let mut expected = vec![first_path, second_path];
        expected.sort_by_key(LocalOrRemotePath::display_path);
        assert_eq!(paths, expected);

        skill_watcher_handle.read(&app, |watcher, _| {
            assert!(!watcher.project_skill_files_by_repo.contains_key(&repo_id));
        });
    });
}

#[test]
fn test_stale_project_skill_refresh_result_is_ignored() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill = create_skill_file(&temp_dir, "stale", "Stale skill", "Old content");
        let repo_id = RepositoryIdentifier::try_local(temp_dir.path()).unwrap();

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            let stale_generation = skill_watcher.advance_project_skill_refresh_generation(&repo_id);
            skill_watcher.advance_project_skill_refresh_generation(&repo_id);
            skill_watcher.emit_project_skills_if_current(
                &repo_id,
                stale_generation,
                vec![skill],
                ctx,
            );
        });

        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn test_removing_project_repo_invalidates_pending_refresh_result() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill = create_skill_file(&temp_dir, "removed", "Removed skill", "Old content");
        let repo_id = RepositoryIdentifier::try_local(temp_dir.path()).unwrap();

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            let pending_generation =
                skill_watcher.advance_project_skill_refresh_generation(&repo_id);
            skill_watcher.remove_project_skills_for_repo(&repo_id);
            skill_watcher.emit_project_skills_if_current(
                &repo_id,
                pending_generation,
                vec![skill],
                ctx,
            );
        });

        assert!(rx.try_recv().is_err());
    });
}

#[test]
#[cfg(unix)]
fn test_refresh_project_skills_for_repo_loads_indexed_and_symlinked_skill_directories() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        let repo_metadata_handle = app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let repo_dir = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        let indexed_skill = create_skill_file(
            &repo_dir,
            "indexed-skill",
            "Indexed skill",
            "Indexed content",
        );
        let target_skill = create_skill_file(
            &target_dir,
            "linked-skill",
            "Linked skill",
            "Linked content",
        );
        let repo = repo_dir.path().to_path_buf();
        let symlink_parent = repo.join(".agents/skills");
        fs::create_dir_all(&symlink_parent).unwrap();
        let symlink_skill_dir = symlink_parent.join("linked-skill");
        std::os::unix::fs::symlink(
            target_skill.path.to_local_path().unwrap().parent().unwrap(),
            &symlink_skill_dir,
        )
        .unwrap();

        let mut expected_skill = target_skill;
        expected_skill.path = LocalOrRemotePath::Local(symlink_skill_dir.join("SKILL.md"));

        let repo_id = RepositoryIdentifier::try_local(&repo).unwrap();
        let repo_key = StandardizedPath::try_from_local(&repo).unwrap();
        repo_metadata_handle.update(&mut app, |model, ctx| {
            model.insert_test_state(
                repo_key.clone(),
                project_state(&repo, Some(&indexed_skill)),
                ctx,
            );
            let mut standing_results = project_standing_results(&repo, Some(&indexed_skill));
            standing_results.insert_project_skill(StandingQueryContent::file(
                StandardizedPath::try_from_local(&skill_local_path(&expected_skill)).unwrap(),
            ));
            model.insert_test_standing_results(repo_key, standing_results, ctx);
        });

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.refresh_project_skills_for_repo(&repo_id, ctx);
        });
        let SkillWatcherEvent::SkillsAdded { mut skills } = rx.recv().await.unwrap() else {
            panic!("Expected SkillsAdded event");
        };
        skills.sort_by_key(|skill| skill.path.display_path());
        let mut expected = vec![indexed_skill, expected_skill];
        expected.sort_by_key(|skill| skill.path.display_path());
        assert_eq!(skills, expected);
    });
}

#[test]
fn test_refresh_project_skills_for_repo_uses_repo_metadata_without_fallback_watcher() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        let repo_metadata_handle = app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill = create_skill_file(&temp_dir, "metadata-skill", "Metadata skill", "Content");
        let repo = temp_dir.path().to_path_buf();
        let repo_id = RepositoryIdentifier::try_local(&repo).unwrap();
        let repo_key = StandardizedPath::try_from_local(&repo).unwrap();

        repo_metadata_handle.update(&mut app, |model, ctx| {
            model.insert_test_state(repo_key.clone(), project_state(&repo, Some(&skill)), ctx);
            model.insert_test_standing_results(
                repo_key,
                project_standing_results(&repo, Some(&skill)),
                ctx,
            );
        });
        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.refresh_project_skills_for_repo(&repo_id, ctx);
            assert!(skill_watcher.failed_local_project_watchers.is_empty());
        });

        assert_eq!(
            rx.recv().await.unwrap(),
            SkillWatcherEvent::SkillsAdded {
                skills: vec![skill]
            }
        );
    });
}

#[test]
fn test_local_project_fallback_scans_filesystem_when_repo_metadata_fails() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let repo = dunce::canonicalize(temp_dir.path()).unwrap();
        let root_skill =
            create_skill_file_in_directory(&repo, "root-skill", "Root skill", "Root content");
        let subdir = repo.join("packages/frontend");
        let subdir_skill =
            create_skill_file_in_directory(&subdir, "frontend-skill", "Frontend skill", "Content");

        let repo_id = RepositoryIdentifier::try_local(&repo).unwrap();
        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.fallback_to_local_project_watcher(&repo_id, ctx);
            assert!(skill_watcher.failed_local_project_watchers.is_empty());
        });

        let SkillWatcherEvent::SkillsAdded { mut skills } = rx.recv().await.unwrap() else {
            panic!("Expected SkillsAdded event");
        };
        skills.sort_by_key(|skill| skill.path.display_path());
        let mut expected = vec![root_skill, subdir_skill];
        expected.sort_by_key(|skill| skill.path.display_path());
        assert_eq!(skills, expected);
    });
}

#[test]
#[cfg(unix)]
fn test_local_project_fallback_initial_scan_loads_symlinked_skill_directory() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let repo_dir = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        let target_skill = create_skill_file(
            &target_dir,
            "fallback-linked-skill",
            "Fallback linked skill",
            "Linked content",
        );
        let repo = dunce::canonicalize(repo_dir.path()).unwrap();
        let symlink_parent = repo.join(".agents/skills");
        fs::create_dir_all(&symlink_parent).unwrap();
        let symlink_skill_dir = symlink_parent.join("fallback-linked-skill");
        std::os::unix::fs::symlink(
            skill_local_path(&target_skill).parent().unwrap(),
            &symlink_skill_dir,
        )
        .unwrap();

        let mut expected_skill = target_skill;
        expected_skill.path = LocalOrRemotePath::Local(symlink_skill_dir.join("SKILL.md"));

        let repo_id = RepositoryIdentifier::try_local(&repo).unwrap();
        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.fallback_to_local_project_watcher(&repo_id, ctx);
        });

        assert_eq!(
            rx.recv().await.unwrap(),
            SkillWatcherEvent::SkillsAdded {
                skills: vec![expected_skill]
            }
        );
    });
}
#[test]
fn test_local_project_fallback_update_reuses_repository_update_handler() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill = create_skill_file(&temp_dir, "fallback-update", "Fallback update", "Content");
        let update = RepositoryUpdate {
            added: HashSet::new(),
            modified: HashSet::from([TargetFile::new(skill_local_path(&skill), false)]),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_message(
                SkillRepositoryMessage::ProjectRepositoryUpdate { update },
                ctx,
            );
        });

        assert_eq!(
            rx.recv().await.unwrap(),
            SkillWatcherEvent::SkillsAdded {
                skills: vec![skill]
            }
        );
    });
}

#[test]
fn test_local_project_fallback_directory_addition_scans_filesystem() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("packages/frontend");
        let skill =
            create_skill_file_in_directory(&new_dir, "fallback-dir", "Fallback dir", "Content");
        let update = RepositoryUpdate {
            added: HashSet::from([TargetFile::new(new_dir, false)]),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_message(
                SkillRepositoryMessage::ProjectRepositoryUpdate { update },
                ctx,
            );
        });

        assert_eq!(
            rx.recv().await.unwrap(),
            SkillWatcherEvent::SkillsAdded {
                skills: vec![skill]
            }
        );
    });
}
#[test]
fn test_handle_repository_update_skill_modified() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill = create_skill_file(&temp_dir, "test", "Test skill", "Test content");

        let update = RepositoryUpdate {
            added: HashSet::new(),
            modified: HashSet::from([TargetFile::new(skill_local_path(&skill), false)]),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        let event = rx.recv().await.unwrap();
        assert_eq!(
            event,
            SkillWatcherEvent::SkillsAdded {
                skills: vec![skill]
            }
        );
    });
}

#[test]
fn test_handle_repository_update_skill_deleted() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill = create_skill_file(&temp_dir, "test", "Test skill", "Test content");

        let update = RepositoryUpdate {
            added: HashSet::new(),
            modified: HashSet::new(),
            deleted: HashSet::from([TargetFile::new(skill_local_path(&skill), false)]),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        let event = rx.recv().await.unwrap();
        assert_eq!(
            event,
            SkillWatcherEvent::SkillsDeleted {
                paths: vec![skill.path]
            }
        );
    });
}

#[test]
fn test_handle_repository_update_multiple_skills_deleted() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill_a = create_skill_file(&temp_dir, "skill-a", "Skill A", "Content A");
        let skill_b = create_skill_file(&temp_dir, "skill-b", "Skill B", "Content B");

        let update = RepositoryUpdate {
            added: HashSet::new(),
            modified: HashSet::new(),
            deleted: HashSet::from([
                TargetFile::new(skill_local_path(&skill_a), false),
                TargetFile::new(skill_local_path(&skill_b), false),
            ]),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        let event = rx.recv().await.unwrap();
        let SkillWatcherEvent::SkillsDeleted { mut paths } = event else {
            panic!("Expected SkillsDeleted event");
        };
        paths.sort_by_key(LocalOrRemotePath::display_path);
        let mut expected = vec![skill_a.path, skill_b.path];
        expected.sort_by_key(LocalOrRemotePath::display_path);
        assert_eq!(paths, expected);
    });
}

#[test]
fn test_handle_repository_update_skill_moved() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let old_skill = create_skill_file(&temp_dir, "old-skill", "Old skill", "Old content");
        let new_skill = create_skill_file(&temp_dir, "new-skill", "New skill", "New content");

        // moved is HashMap<to_target, from_target>
        let update = RepositoryUpdate {
            added: HashSet::new(),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::from([(
                TargetFile::new(skill_local_path(&new_skill), false),
                TargetFile::new(skill_local_path(&old_skill), false),
            )]),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        // Collect both events: SkillsAdded for the new location and SkillsDeleted for the old
        let event1 = rx.recv().await.unwrap();
        let event2 = rx.recv().await.unwrap();

        let added_event = SkillWatcherEvent::SkillsAdded {
            skills: vec![new_skill],
        };
        let deleted_event = SkillWatcherEvent::SkillsDeleted {
            paths: vec![old_skill.path],
        };
        assert!(
            (event1 == added_event && event2 == deleted_event)
                || (event1 == deleted_event && event2 == added_event),
            "Expected one SkillsAdded and one SkillsDeleted event; got: {event1:?} and {event2:?}"
        );
    });
}

// ============================================================================
// Tests for project skill refreshes
// ============================================================================

#[test]
fn test_handle_repository_update_non_skill_directory_added_does_not_emit_project_event() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new-feature");
        fs::create_dir_all(&new_dir).unwrap();

        let update = RepositoryUpdate {
            added: HashSet::from([TargetFile::new(new_dir, false)]),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        assert!(rx.try_recv().is_err());
    });
}

fn project_state(repo: &std::path::Path, skill: Option<&ParsedSkill>) -> FileTreeState {
    let children = if let Some(skill) = skill {
        let skill_path = skill_local_path(skill);
        let skill_file = Entry::File(FileMetadata::new(skill_path.clone(), false));
        let skill_dir = Entry::Directory(DirectoryEntry {
            path: StandardizedPath::try_from_local(skill_path.parent().unwrap()).unwrap(),
            children: vec![skill_file],
            ignored: false,
            loaded: true,
        });
        let skills_dir = Entry::Directory(DirectoryEntry {
            path: StandardizedPath::try_from_local(&repo.join(".agents/skills")).unwrap(),
            children: vec![skill_dir],
            ignored: false,
            loaded: true,
        });
        let agents_dir = Entry::Directory(DirectoryEntry {
            path: StandardizedPath::try_from_local(&repo.join(".agents")).unwrap(),
            children: vec![skills_dir],
            ignored: false,
            loaded: true,
        });
        vec![agents_dir]
    } else {
        Vec::new()
    };

    let root = Entry::Directory(DirectoryEntry {
        path: StandardizedPath::try_from_local(repo).unwrap(),
        children,
        ignored: false,
        loaded: true,
    });
    FileTreeState::new(root, Vec::new(), None)
}

fn project_standing_results(
    repo: &std::path::Path,
    skill: Option<&ParsedSkill>,
) -> StandingQueryResults {
    let mut delta = StandingQueryResultsDelta {
        upserted_project_skills: vec![StandingQueryContent::directory(
            StandardizedPath::try_from_local(&repo.join(".agents/skills")).unwrap(),
        )],
        ..StandingQueryResultsDelta::default()
    };
    if let Some(skill) = skill {
        delta
            .upserted_project_skills
            .push(StandingQueryContent::file(
                StandardizedPath::try_from_local(&skill_local_path(skill)).unwrap(),
            ));
    }
    let mut results = StandingQueryResults::default();
    results.apply_delta(&delta);
    results
}

#[test]
fn test_refresh_project_skills_for_repo_removes_missing_project_skill_paths() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        let repo_metadata_handle = app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let skill = create_skill_file(&temp_dir, "test", "Test skill", "Test content");
        let repo = temp_dir.path().to_path_buf();
        let repo_id = RepositoryIdentifier::try_local(&repo).unwrap();
        let repo_key = StandardizedPath::try_from_local(&repo).unwrap();

        repo_metadata_handle.update(&mut app, |model, ctx| {
            model.insert_test_state(repo_key.clone(), project_state(&repo, Some(&skill)), ctx);
            model.insert_test_standing_results(
                repo_key.clone(),
                project_standing_results(&repo, Some(&skill)),
                ctx,
            );
        });
        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.refresh_project_skills_for_repo(&repo_id, ctx);
        });

        assert_eq!(
            rx.recv().await.unwrap(),
            SkillWatcherEvent::SkillsAdded {
                skills: vec![skill.clone()]
            }
        );

        repo_metadata_handle.update(&mut app, |model, ctx| {
            model.insert_test_state(repo_key.clone(), project_state(&repo, None), ctx);
            model.insert_test_standing_results(
                repo_key,
                project_standing_results(&repo, None),
                ctx,
            );
        });
        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.refresh_project_skills_for_repo(&repo_id, ctx);
        });

        assert_eq!(
            rx.recv().await.unwrap(),
            SkillWatcherEvent::SkillsDeleted {
                paths: vec![skill.path]
            }
        );
    });
}

/// Regression test for the multi-provider shared-canonical case: when two provider
/// parents (e.g. `~/.agents` and `~/.claude`) both symlink to the same directory,
/// a single canonical event must fan out to all originals so each provider's view
/// of the skill stays in sync.
#[test]
fn test_handle_repository_update_fans_out_to_all_originals_for_shared_canonical() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        // Create the same skill file under both .agents and .claude so parse_skill
        // succeeds when called on either translated path.
        let skill_agents = create_skill_file(&temp_dir, "test", "Test skill", "Test content");
        let skill_content = fs::read_to_string(skill_local_path(&skill_agents)).unwrap();
        let claude_skill_dir = temp_dir.path().join(".claude").join("skills").join("test");
        fs::create_dir_all(&claude_skill_dir).unwrap();
        let claude_skill_path = claude_skill_dir.join("SKILL.md");
        fs::write(&claude_skill_path, skill_content).unwrap();

        let agents_provider = temp_dir.path().join(".agents");
        let claude_provider = temp_dir.path().join(".claude");
        let canonical_provider = temp_dir.path().join("shared-dotfiles");

        // Both originals resolve to the same canonical.
        skill_watcher_handle.update(&mut app, |watcher, _ctx| {
            let entry = watcher
                .home_provider_canonical_to_originals
                .entry(canonical_provider.clone())
                .or_default();
            entry.insert(agents_provider);
            entry.insert(claude_provider);
        });

        let canonical_skill_path = canonical_provider
            .join("skills")
            .join("test")
            .join("SKILL.md");
        let update = RepositoryUpdate {
            added: HashSet::from([TargetFile::new(canonical_skill_path, false)]),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        // Two events — one per original provider. HashSet iteration is unordered,
        // so collect paths into a set for order-independent comparison.
        let mut paths_seen: HashSet<_> = HashSet::new();
        for _ in 0..2 {
            match rx.recv().await.unwrap() {
                SkillWatcherEvent::SkillsAdded { skills } => {
                    assert_eq!(skills.len(), 1);
                    paths_seen.insert(skills[0].path.clone());
                }
                other => panic!("Expected SkillsAdded, got {:?}", other),
            }
        }
        assert!(paths_seen.contains(&skill_agents.path));
        assert!(paths_seen.contains(&LocalOrRemotePath::Local(claude_skill_path)));
        // Pin event cardinality: exactly two events from fan-out, no extras.
        assert!(rx.try_recv().is_err());
    });
}

/// When a `moved` event has mismatched canonical fan-out (e.g. `mv` from outside
/// a provider into a shared-canonical provider with two originals), the move
/// can't be paired but must not be dropped — `notify` can report a cross-boundary
/// rename without a separate add, so dropping would lose the destination skill.
/// This test pins that the destination side is salvaged as an add (fanned out
/// to all originals) and the source side as a delete.
#[test]
fn test_handle_repository_update_salvages_mismatched_moves_as_add_and_delete() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        // Two providers symlinked to the same canonical (shared-canonical setup).
        let skill_content = r#"---
name: test
description: Test skill
---
Test content
"#;
        let agents_skill_dir = temp_dir.path().join(".agents").join("skills").join("test");
        fs::create_dir_all(&agents_skill_dir).unwrap();
        let agents_skill_path = agents_skill_dir.join("SKILL.md");
        fs::write(&agents_skill_path, skill_content).unwrap();
        let claude_skill_dir = temp_dir.path().join(".claude").join("skills").join("test");
        fs::create_dir_all(&claude_skill_dir).unwrap();
        let claude_skill_path = claude_skill_dir.join("SKILL.md");
        fs::write(&claude_skill_path, skill_content).unwrap();

        let agents_provider = temp_dir.path().join(".agents");
        let claude_provider = temp_dir.path().join(".claude");
        let canonical_provider = temp_dir.path().join("shared-dotfiles");

        skill_watcher_handle.update(&mut app, |watcher, _ctx| {
            let entry = watcher
                .home_provider_canonical_to_originals
                .entry(canonical_provider.clone())
                .or_default();
            entry.insert(agents_provider.clone());
            entry.insert(claude_provider.clone());
        });

        // Cross-boundary rename: source outside any canonical, destination
        // inside the shared-canonical provider. Fan-out lengths: 1 (source
        // pass-through) vs 2 (destination expands to both originals).
        let outside_source = temp_dir.path().join("outside-source-SKILL.md");
        let canonical_destination = canonical_provider
            .join("skills")
            .join("test")
            .join("SKILL.md");
        let mut moved = HashMap::new();
        moved.insert(
            TargetFile::new(canonical_destination, false),
            TargetFile::new(outside_source.clone(), false),
        );
        let update = RepositoryUpdate {
            added: HashSet::new(),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved,
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |skill_watcher, ctx| {
            skill_watcher.handle_repository_update(&update, ctx);
        });

        // Two SkillsAdded events expected (one per original at the destination).
        // The source side becomes a SkillsDeleted with the un-translated path.
        let mut added_paths: HashSet<_> = HashSet::new();
        let mut deleted_paths: HashSet<_> = HashSet::new();
        for _ in 0..3 {
            match rx.recv().await.unwrap() {
                SkillWatcherEvent::SkillsAdded { skills } => {
                    assert_eq!(skills.len(), 1);
                    added_paths.insert(skills[0].path.clone());
                }
                SkillWatcherEvent::SkillsDeleted { paths } => {
                    deleted_paths.extend(paths);
                }
            }
        }

        assert!(added_paths.contains(&LocalOrRemotePath::Local(agents_skill_path)));
        assert!(added_paths.contains(&LocalOrRemotePath::Local(claude_skill_path)));
        assert!(deleted_paths.contains(&LocalOrRemotePath::Local(outside_source)));
        // Pin cardinality: no extra events beyond 2 adds + 1 delete.
        assert!(rx.try_recv().is_err());
    });
}

/// Pins the deepest-prefix matching invariant for `translate_canonical_to_original_paths`.
/// `HashMap` iteration order is unstable, so a first-match-wins implementation would
/// translate the same input two different ways across runs when canonicals nest.
/// Deepest match must always win.
#[test]
fn test_translate_canonical_picks_deepest_prefix_match() {
    let (tx, _rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let shallow_canonical = temp_dir.path().join("shared");
        let deep_canonical = shallow_canonical.join("nested");
        let shallow_original = temp_dir.path().join(".shallow-orig");
        let deep_original = temp_dir.path().join(".deep-orig");

        skill_watcher_handle.update(&mut app, |watcher, _ctx| {
            watcher
                .home_provider_canonical_to_originals
                .entry(shallow_canonical.clone())
                .or_default()
                .insert(shallow_original.clone());
            watcher
                .home_provider_canonical_to_originals
                .entry(deep_canonical.clone())
                .or_default()
                .insert(deep_original.clone());
        });

        let input = deep_canonical.join("skills").join("test").join("SKILL.md");

        skill_watcher_handle.read(&app, |watcher, _ctx| {
            let translated = watcher.translate_canonical_to_original_paths(&input);
            // Deepest match: rel = "skills/test/SKILL.md" joined under .deep-orig.
            let expected = deep_original.join("skills").join("test").join("SKILL.md");
            assert_eq!(translated, vec![expected]);
            // The shallow translation (rel = "nested/skills/...") must NOT be the result.
            let shallow_wrong = shallow_original
                .join("nested")
                .join("skills")
                .join("test")
                .join("SKILL.md");
            assert!(!translated.contains(&shallow_wrong));
        });
    });
}

/// When the canonical target of a symlinked provider is itself a registered
/// provider (e.g. `~/.claude` → `~/.agents`), translation must keep the
/// identity path alongside the symlink-side original — otherwise the real
/// provider's own skills go stale while every event under its directory is
/// re-attributed to the alias.
#[test]
fn test_translate_keeps_identity_when_canonical_is_itself_a_provider() {
    let (tx, _rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let agents_provider = temp_dir.path().join(".agents");
        let claude_provider = temp_dir.path().join(".claude");
        fs::create_dir_all(&agents_provider).unwrap();

        skill_watcher_handle.update(&mut app, |watcher, ctx| {
            // `.claude` resolved to `.agents` at registration…
            watcher
                .home_provider_canonical_to_originals
                .entry(agents_provider.clone())
                .or_default()
                .insert(claude_provider.clone());

            // …and `.agents` is itself a registered provider watch.
            let std_path = StandardizedPath::from_local_canonicalized(&agents_provider).unwrap();
            let repo_handle = DirectoryWatcher::handle(ctx)
                .update(ctx, |directory_watcher, ctx| {
                    directory_watcher.add_directory(std_path, ctx)
                })
                .unwrap();
            let start = repo_handle.update(ctx, |repo, ctx| {
                repo.start_watching(Box::new(NoopRepositorySubscriber), ctx)
            });
            watcher
                .home_provider_watchers
                .insert(agents_provider.clone(), (repo_handle, start.subscriber_id));
        });

        skill_watcher_handle.read(&app, |watcher, _ctx| {
            let input = agents_provider.join("skills").join("test").join("SKILL.md");
            let translated: HashSet<_> = watcher
                .translate_canonical_to_original_paths(&input)
                .into_iter()
                .collect();
            let expected: HashSet<_> = [
                claude_provider.join("skills").join("test").join("SKILL.md"),
                agents_provider.join("skills").join("test").join("SKILL.md"),
            ]
            .into_iter()
            .collect();
            assert_eq!(translated, expected);
        });
    });
}

#[test]
fn test_handle_home_files_changed_keeps_remaining_original_for_shared_canonical() {
    let (tx, rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        let agents_provider = temp_dir.path().join(".agents");
        let claude_provider = temp_dir.path().join(".claude");
        let canonical_provider = temp_dir.path().join("shared-dotfiles");
        fs::create_dir_all(&canonical_provider).unwrap();

        let skill_content = r#"---
name: test
description: Test skill
---
Test content
"#;
        let claude_skill_dir = claude_provider.join("skills").join("test");
        fs::create_dir_all(&claude_skill_dir).unwrap();
        let claude_skill_path = claude_skill_dir.join("SKILL.md");
        fs::write(&claude_skill_path, skill_content).unwrap();

        skill_watcher_handle.update(&mut app, |watcher, ctx| {
            let canonical_path =
                StandardizedPath::from_local_canonicalized(&canonical_provider).unwrap();
            let repo_handle = DirectoryWatcher::handle(ctx)
                .update(ctx, |directory_watcher, ctx| {
                    directory_watcher.add_directory(canonical_path, ctx)
                })
                .unwrap();

            let agents_start = repo_handle.update(ctx, |repo, ctx| {
                repo.start_watching(Box::new(NoopRepositorySubscriber), ctx)
            });
            let claude_start = repo_handle.update(ctx, |repo, ctx| {
                repo.start_watching(Box::new(NoopRepositorySubscriber), ctx)
            });

            watcher.home_provider_watchers.insert(
                agents_provider.clone(),
                (repo_handle.clone(), agents_start.subscriber_id),
            );
            watcher.home_provider_watchers.insert(
                claude_provider.clone(),
                (repo_handle, claude_start.subscriber_id),
            );

            let originals = watcher
                .home_provider_canonical_to_originals
                .entry(canonical_provider.clone())
                .or_default();
            originals.insert(agents_provider.clone());
            originals.insert(claude_provider.clone());
        });

        let delete_event = watcher::BulkFilesystemWatcherEvent {
            deleted: HashSet::from([agents_provider.clone()]),
            ..Default::default()
        };
        skill_watcher_handle.update(&mut app, |watcher, ctx| {
            watcher.handle_home_files_changed(&delete_event, ctx);
        });

        assert_eq!(
            rx.recv().await.unwrap(),
            SkillWatcherEvent::SkillsDeleted {
                paths: vec![LocalOrRemotePath::Local(agents_provider.clone())]
            }
        );

        skill_watcher_handle.read(&app, |watcher, ctx| {
            assert!(!watcher
                .home_provider_watchers
                .contains_key(&agents_provider));
            let (repo_handle, _) = watcher
                .home_provider_watchers
                .get(&claude_provider)
                .expect("remaining provider watcher should stay registered");
            assert_eq!(repo_handle.read(ctx, |repo, _| repo.watcher_count()), 1);

            let originals = watcher
                .home_provider_canonical_to_originals
                .get(&canonical_provider)
                .expect("canonical entry should remain for the remaining original");
            assert_eq!(originals.len(), 1);
            assert!(!originals.contains(&agents_provider));
            assert!(originals.contains(&claude_provider));
        });

        let canonical_skill_path = canonical_provider
            .join("skills")
            .join("test")
            .join("SKILL.md");
        let update = RepositoryUpdate {
            added: HashSet::from([TargetFile::new(canonical_skill_path, false)]),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
            remote_ref_updated: false,
        };

        skill_watcher_handle.update(&mut app, |watcher, ctx| {
            watcher.handle_repository_update(&update, ctx);
        });

        let event = rx.recv().await.unwrap();
        let SkillWatcherEvent::SkillsAdded { skills } = event else {
            panic!("Expected SkillsAdded event");
        };
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].path, LocalOrRemotePath::Local(claude_skill_path));
        assert!(rx.try_recv().is_err());
    });
}

/// Regression test for the symlink-retarget case (`ln -sfn new-target ~/.agents`):
/// re-registering the same original provider with a new canonical target must
/// purge the previous canonical→original entry. Otherwise the stale, deeper
/// mapping wins deepest-prefix matching for events under a still-existing old
/// target subdir and translates them into phantom provider paths.
#[cfg(unix)]
#[test]
fn test_watch_home_provider_path_reregistration_purges_stale_canonical_mapping() {
    let (tx, _rx) = async_channel::unbounded();

    App::test((), |mut app| async move {
        app.add_singleton_model(DirectoryWatcher::new_for_testing);
        app.add_singleton_model(|_| DetectedRepositories::default());
        app.add_singleton_model(RepoMetadataModel::new);
        let skill_watcher_handle = app.add_model(|ctx| SkillWatcher::new_for_testing(ctx, tx));

        let temp_dir = TempDir::new().unwrap();
        // First target is dotfiles/set-a; the retarget points at dotfiles, so
        // the old target stays on disk *inside* the new one — the harmful
        // shape, because the stale mapping is deeper than the fresh one.
        let dotfiles = temp_dir.path().join("dotfiles");
        let set_a = dotfiles.join("set-a");
        fs::create_dir_all(&set_a).unwrap();
        let provider = temp_dir.path().join(".agents");
        std::os::unix::fs::symlink(&set_a, &provider).unwrap();

        // Canonicalized forms (macOS tempdirs resolve through /private/var).
        let canonical_set_a = dunce::canonicalize(&set_a).unwrap();
        let canonical_dotfiles = dunce::canonicalize(&dotfiles).unwrap();

        skill_watcher_handle.update(&mut app, |watcher, ctx| {
            let message_tx = watcher.repository_message_tx.clone();
            SkillWatcher::watch_home_provider_path(
                &provider,
                &message_tx,
                &mut watcher.home_provider_watchers,
                &mut watcher.home_provider_canonical_to_originals,
                ctx,
            );
        });

        skill_watcher_handle.read(&app, |watcher, _ctx| {
            let originals = watcher
                .home_provider_canonical_to_originals
                .get(&canonical_set_a)
                .expect("first registration should map the canonical target");
            assert!(originals.contains(&provider));
        });
        let (old_repo_handle, old_subscriber_id) = skill_watcher_handle.read(&app, |watcher, _| {
            watcher
                .home_provider_watchers
                .get(&provider)
                .expect("first registration should store the provider watcher")
                .clone()
        });

        // Retarget the provider symlink onto the parent directory.
        fs::remove_file(&provider).unwrap();
        std::os::unix::fs::symlink(&dotfiles, &provider).unwrap();

        skill_watcher_handle.update(&mut app, |watcher, ctx| {
            let message_tx = watcher.repository_message_tx.clone();
            SkillWatcher::watch_home_provider_path(
                &provider,
                &message_tx,
                &mut watcher.home_provider_watchers,
                &mut watcher.home_provider_canonical_to_originals,
                ctx,
            );
        });
        let (new_repo_handle, new_subscriber_id) = skill_watcher_handle.read(&app, |watcher, _| {
            watcher
                .home_provider_watchers
                .get(&provider)
                .expect("re-registration should replace the provider watcher")
                .clone()
        });
        assert!(old_repo_handle != new_repo_handle);
        assert_eq!(
            old_repo_handle.read(&app, |repo, _| repo.watcher_count()),
            0
        );
        assert_eq!(
            new_repo_handle.read(&app, |repo, _| repo.watcher_count()),
            1
        );

        // A late failure from the superseded registration must not roll back
        // the live mapping installed by the newer registration.
        skill_watcher_handle.update(&mut app, |watcher, ctx| {
            watcher.rollback_home_provider_watch_registration(
                &provider,
                &old_repo_handle,
                old_subscriber_id,
                ctx,
            );
        });

        skill_watcher_handle.read(&app, |watcher, _ctx| {
            // The stale mapping is purged; only the live target remains.
            assert!(!watcher
                .home_provider_canonical_to_originals
                .contains_key(&canonical_set_a));
            assert!(watcher.home_provider_watchers.get(&provider).is_some_and(
                |(repo_handle, subscriber_id)| {
                    repo_handle == &new_repo_handle && *subscriber_id == new_subscriber_id
                }
            ));
            let originals = watcher
                .home_provider_canonical_to_originals
                .get(&canonical_dotfiles)
                .expect("re-registration should map the new canonical target");
            assert!(originals.contains(&provider));

            // The harm the purge prevents: an event under the old target dir
            // (still on disk inside the new target) must translate through the
            // live mapping (`.agents/set-a/...`), not the stale deeper one
            // (`.agents/...`).
            let event_path = canonical_set_a.join("skills").join("test").join("SKILL.md");
            let translated = watcher.translate_canonical_to_original_paths(&event_path);
            assert_eq!(
                translated,
                vec![provider
                    .join("set-a")
                    .join("skills")
                    .join("test")
                    .join("SKILL.md")]
            );
        });
    });
}
