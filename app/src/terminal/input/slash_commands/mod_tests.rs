use crate::features::FeatureFlag;
use crate::search::slash_command_menu::static_commands::{commands, Availability};
use crate::workspace::tab_settings::{TabColorSlot, TabColorSlotLabels};
use warp_core::ui::theme::AnsiColorIdentifier;

use super::{
    parse_name_window_argument, parse_rename_tab_color_argument, parse_set_tab_color_argument,
    NameWindowCommandArgument, RenameTabColorCommandArgument,
};
use crate::tab::SelectedTabColor;
use crate::ui_components::color_dot::TAB_COLOR_OPTIONS;

const BASELINE_AVAILABILITY: Availability = Availability::AGENT_VIEW
    .union(Availability::AI_ENABLED)
    .union(Availability::NO_LRC_CONTROL);

#[test]
fn name_window_argument_sets_trimmed_name() {
    assert_eq!(
        parse_name_window_argument(Some("  Production Logs  ")),
        Ok(NameWindowCommandArgument::Set(
            "Production Logs".to_string()
        ))
    );
}

#[test]
fn name_window_argument_clears_only_on_exact_clear_flag() {
    assert_eq!(
        parse_name_window_argument(Some("--clear")),
        Ok(NameWindowCommandArgument::Clear)
    );
    assert_eq!(
        parse_name_window_argument(Some("--clear later")),
        Ok(NameWindowCommandArgument::Set("--clear later".to_string()))
    );
}

#[test]
fn name_window_argument_rejects_missing_or_blank_name() {
    assert!(parse_name_window_argument(None).is_err());
    assert!(parse_name_window_argument(Some("   ")).is_err());
}

#[test]
fn rename_tab_color_argument_sets_trimmed_label() {
    assert_eq!(
        parse_rename_tab_color_argument(Some(" blue   GOAL: primary  "), &TAB_COLOR_OPTIONS,),
        Ok(RenameTabColorCommandArgument::Set {
            slot: TabColorSlot::Blue,
            label: "GOAL: primary".to_owned(),
        })
    );
}

#[test]
fn rename_tab_color_argument_sets_quoted_default_label() {
    assert_eq!(
        parse_rename_tab_color_argument(Some(" default   \"Inactive\"  "), &TAB_COLOR_OPTIONS,),
        Ok(RenameTabColorCommandArgument::Set {
            slot: TabColorSlot::Default,
            label: "Inactive".to_owned(),
        })
    );
}

#[test]
fn rename_tab_color_argument_clears_only_on_exact_clear_flag_after_color() {
    assert_eq!(
        parse_rename_tab_color_argument(Some("blue --clear"), &TAB_COLOR_OPTIONS),
        Ok(RenameTabColorCommandArgument::Clear {
            slot: TabColorSlot::Blue,
        })
    );
    assert_eq!(
        parse_rename_tab_color_argument(Some("default --clear"), &TAB_COLOR_OPTIONS),
        Ok(RenameTabColorCommandArgument::Clear {
            slot: TabColorSlot::Default,
        })
    );
    assert_eq!(
        parse_rename_tab_color_argument(Some("blue --clear later"), &TAB_COLOR_OPTIONS),
        Ok(RenameTabColorCommandArgument::Set {
            slot: TabColorSlot::Blue,
            label: "--clear later".to_owned(),
        })
    );
}

#[test]
fn rename_tab_color_argument_rejects_missing_unknown_or_blank_label() {
    assert!(parse_rename_tab_color_argument(None, &TAB_COLOR_OPTIONS).is_err());
    assert!(parse_rename_tab_color_argument(Some("   "), &TAB_COLOR_OPTIONS).is_err());
    assert!(parse_rename_tab_color_argument(Some("black Work"), &TAB_COLOR_OPTIONS).is_err());
    assert!(parse_rename_tab_color_argument(Some("blue   "), &TAB_COLOR_OPTIONS).is_err());
}

#[test]
fn set_tab_color_argument_resolves_raw_color_custom_label_and_none() {
    let labels = TabColorSlotLabels::default()
        .with_label(
            AnsiColorIdentifier::Blue,
            "GOAL: primary",
            &TAB_COLOR_OPTIONS,
        )
        .expect("blue label should save")
        .with_label(TabColorSlot::Default, "Inactive", &TAB_COLOR_OPTIONS)
        .expect("default label should save");

    assert_eq!(
        parse_set_tab_color_argument(Some("blue"), &labels),
        Ok(SelectedTabColor::Color(AnsiColorIdentifier::Blue))
    );
    assert_eq!(
        parse_set_tab_color_argument(Some("  goal: PRIMARY  "), &labels),
        Ok(SelectedTabColor::Color(AnsiColorIdentifier::Blue))
    );
    assert_eq!(
        parse_set_tab_color_argument(Some("none"), &labels),
        Ok(SelectedTabColor::Cleared)
    );
    assert_eq!(
        parse_set_tab_color_argument(Some("default"), &labels),
        Ok(SelectedTabColor::Cleared)
    );
    assert_eq!(
        parse_set_tab_color_argument(Some("inactive"), &labels),
        Ok(SelectedTabColor::Cleared)
    );
    assert!(parse_set_tab_color_argument(Some("goal"), &labels).is_err());
}

#[test]
fn rename_tab_color_rejects_reserved_default_and_color_labels() {
    let labels = TabColorSlotLabels::default();

    for label in ["default", "none", "red", "black", "WHITE"] {
        assert_eq!(
            labels.with_label(TabColorSlot::Blue, label, &TAB_COLOR_OPTIONS),
            Err(crate::workspace::tab_settings::TabColorSlotLabelError::ReservedLabel),
            "{label} should be reserved"
        );
    }
}

#[test]
fn not_cloud_agent_commands_are_only_active_outside_cloud_mode() {
    let local_context = BASELINE_AVAILABILITY | Availability::NOT_CLOUD_AGENT;
    assert!(commands::AGENT.is_active(local_context));
    assert!(commands::NEW.is_active(local_context));

    let cloud_context = BASELINE_AVAILABILITY;
    assert!(!commands::AGENT.is_active(cloud_context));
    assert!(!commands::NEW.is_active(cloud_context));

    let _cloud_mode_input_v2 = FeatureFlag::CloudModeInputV2.override_enabled(true);
    let cloud_mode_v2_context = BASELINE_AVAILABILITY | Availability::CLOUD_MODE_V2_COMPOSER;
    assert!(!commands::AGENT.is_active(cloud_mode_v2_context));
    assert!(!commands::NEW.is_active(cloud_mode_v2_context));
}

#[test]
fn cloud_mode_v2_commands_are_active_only_in_cloud_mode_v2_context() {
    let cloud_context = BASELINE_AVAILABILITY;
    assert!(!commands::HARNESS.is_active(cloud_context));

    let _cloud_mode_input_v2 = FeatureFlag::CloudModeInputV2.override_enabled(true);
    let cloud_mode_v2_context = BASELINE_AVAILABILITY | Availability::CLOUD_MODE_V2_COMPOSER;
    assert!(commands::PLAN.is_active(cloud_mode_v2_context));
    assert!(commands::MODEL.is_active(cloud_mode_v2_context));
    assert!(commands::HARNESS.is_active(cloud_mode_v2_context));
}

#[cfg(all(feature = "local_fs", windows))]
mod windows {
    use std::sync::Arc;

    use super::super::*;
    use crate::terminal::model::session::command_executor::testing::TestCommandExecutor;
    use crate::terminal::model::session::SessionInfo;
    use crate::terminal::shell::ShellType;
    use crate::terminal::ShellLaunchData;

    fn wsl_session() -> Session {
        Session::new(
            SessionInfo::new_for_test().with_shell_type(ShellType::Bash),
            Arc::new(TestCommandExecutor::default()),
        )
        .with_shell_launch_data(ShellLaunchData::WSL {
            distro: "Ubuntu".to_owned(),
        })
    }

    #[test]
    fn open_file_command_converts_wsl_paths_to_host_paths() {
        let session = wsl_session();
        let cases = [
            (
                "/home/ubuntu",
                "subdir/test.txt",
                r"\\WSL$\Ubuntu\home\ubuntu\subdir\test.txt",
                None,
            ),
            (
                "/home/ubuntu/project",
                "../test.txt",
                r"\\WSL$\Ubuntu\home\ubuntu\test.txt",
                None,
            ),
            (
                "/home/ubuntu",
                "subdir/file\\ name.txt",
                r"\\WSL$\Ubuntu\home\ubuntu\subdir\file name.txt",
                None,
            ),
            (
                "/home/ubuntu",
                "subdir/test.txt:4:2",
                r"\\WSL$\Ubuntu\home\ubuntu\subdir\test.txt",
                Some(LineAndColumnArg {
                    line_num: 4,
                    column_num: Some(2),
                }),
            ),
        ];

        for (current_dir, raw_arg, expected_path, expected_line_col) in cases {
            let (path, line_col) = open_file_command_path(&session, current_dir, raw_arg);

            assert_eq!(path, PathBuf::from(expected_path));
            assert_eq!(line_col, expected_line_col);
        }
    }
}
