use warpui::EntityId;

use super::WorkspaceAction;
use crate::pane_group::TerminalPaneId;
use crate::workspace::tab_settings::{
    VerticalTabsDisplayGranularity, VerticalTabsPrimaryInfo, VerticalTabsTabItemMode,
    VerticalTabsViewMode,
};
use crate::workspace::PaneViewLocator;

#[test]
fn vertical_tabs_view_mode_change_does_not_save_workspace_state() {
    assert!(
        !WorkspaceAction::SetVerticalTabsViewMode(VerticalTabsViewMode::Compact)
            .should_save_app_state_on_action()
    );
}

#[test]
fn vertical_tabs_panel_toggle_still_saves_workspace_state() {
    assert!(WorkspaceAction::ToggleVerticalTabsPanel.should_save_app_state_on_action());
}

#[test]
fn settings_popup_toggle_does_not_save_workspace_state() {
    assert!(!WorkspaceAction::ToggleVerticalTabsSettingsPopup.should_save_app_state_on_action());
}

#[test]
fn display_granularity_change_does_not_save_workspace_state() {
    assert!(!WorkspaceAction::SetVerticalTabsDisplayGranularity(
        VerticalTabsDisplayGranularity::Panes
    )
    .should_save_app_state_on_action());
    assert!(!WorkspaceAction::SetVerticalTabsDisplayGranularity(
        VerticalTabsDisplayGranularity::Tabs
    )
    .should_save_app_state_on_action());
}

#[test]
fn tab_item_mode_change_does_not_save_workspace_state() {
    assert!(
        !WorkspaceAction::SetVerticalTabsTabItemMode(VerticalTabsTabItemMode::FocusedSession)
            .should_save_app_state_on_action()
    );
    assert!(
        !WorkspaceAction::SetVerticalTabsTabItemMode(VerticalTabsTabItemMode::Summary)
            .should_save_app_state_on_action()
    );
}

#[test]
fn primary_info_change_does_not_save_workspace_state() {
    assert!(
        !WorkspaceAction::SetVerticalTabsPrimaryInfo(VerticalTabsPrimaryInfo::Command)
            .should_save_app_state_on_action()
    );
    assert!(!WorkspaceAction::SetVerticalTabsPrimaryInfo(
        VerticalTabsPrimaryInfo::WorkingDirectory
    )
    .should_save_app_state_on_action());
    assert!(
        !WorkspaceAction::SetVerticalTabsPrimaryInfo(VerticalTabsPrimaryInfo::Branch)
            .should_save_app_state_on_action()
    );
}

#[test]
fn pane_name_actions_save_workspace_state() {
    let locator = PaneViewLocator {
        pane_group_id: EntityId::new(),
        pane_id: TerminalPaneId::dummy_terminal_pane_id().into(),
    };

    assert!(WorkspaceAction::RenamePane(locator).should_save_app_state_on_action());
    assert!(WorkspaceAction::ResetPaneName(locator).should_save_app_state_on_action());
    // GH-9351: the keyboard-bindable variant must persist app state on the
    // same conditions as the locator-based one, since both ultimately drive
    // `rename_pane` which mutates `pane_configuration`.
    assert!(WorkspaceAction::RenameActivePane.should_save_app_state_on_action());
    // #warp-49: `/rename-pane` sets the name outright with no editor. On the set
    // path the dispatcher's save is the *only* thing that persists it, because
    // the handler deliberately does not dispatch `workspace:save_app` itself.
    // (The clear path also saves inside `clear_pane_name`, which it shares with
    // `ResetPaneName`.)
    assert!(WorkspaceAction::SetPaneName {
        pane_id: Some(locator.pane_id),
        name: Some("API server".to_owned()),
    }
    .should_save_app_state_on_action());
    assert!(WorkspaceAction::SetPaneName {
        pane_id: None,
        name: None,
    }
    .should_save_app_state_on_action());
}
