use settings::Setting;
use warpui::{App, SingletonEntity};

use super::*;
use crate::test_util::settings::initialize_settings_for_tests;
use crate::workspace::header_toolbar_item::HeaderToolbarItemKind;

#[test]
fn color_slot_labels_use_global_settings_sync_path() {
    assert_eq!(
        TabColorSlotLabels::toml_path(),
        Some("appearance.tabs.color_slot_labels")
    );
    assert_eq!(
        TabColorSlotLabels::sync_to_cloud(),
        SyncToCloud::Globally(RespectUserSyncSetting::Yes)
    );
    assert_eq!(DirectoryTabColors::sync_to_cloud(), SyncToCloud::Never);
}

#[test]
fn color_slot_labels_fall_back_to_raw_color_name() {
    let labels = TabColorSlotLabels::default();
    assert_eq!(labels.label_for_slot(TabColorSlot::Default), "Default");
    assert_eq!(
        labels.display_label_for_slot(TabColorSlot::Default),
        "Default"
    );
    assert_eq!(labels.label_for(AnsiColorIdentifier::Blue), "Blue");
    assert_eq!(labels.display_label_for(AnsiColorIdentifier::Blue), "Blue");
}

#[test]
fn color_slot_labels_store_trimmed_unique_color_labels() {
    let labels = TabColorSlotLabels::default()
        .with_label(
            AnsiColorIdentifier::Blue,
            "  GOAL: primary  ",
            &crate::ui_components::color_dot::TAB_COLOR_OPTIONS,
        )
        .expect("blue label should save");

    assert_eq!(labels.label_for(AnsiColorIdentifier::Blue), "GOAL: primary");
    assert_eq!(
        labels.display_label_for(AnsiColorIdentifier::Blue),
        "GOAL: primary (Blue)"
    );
    assert_eq!(
        labels.resolve_label(
            "goal: PRIMARY",
            &crate::ui_components::color_dot::TAB_COLOR_OPTIONS
        ),
        Some(AnsiColorIdentifier::Blue)
    );
    assert_eq!(
        labels.resolve_label("goal", &crate::ui_components::color_dot::TAB_COLOR_OPTIONS),
        None
    );
}

#[test]
fn color_slot_labels_store_trimmed_default_label() {
    let labels = TabColorSlotLabels::default()
        .with_label(
            TabColorSlot::Default,
            "  Inactive  ",
            &crate::ui_components::color_dot::TAB_COLOR_OPTIONS,
        )
        .expect("default label should save");

    assert_eq!(labels.label_for_slot(TabColorSlot::Default), "Inactive");
    assert_eq!(
        labels.display_label_for_slot(TabColorSlot::Default),
        "Inactive (Default)"
    );
    assert_eq!(
        labels.resolve_slot_label(
            "inactive",
            &crate::ui_components::color_dot::TAB_COLOR_OPTIONS
        ),
        Some(TabColorSlot::Default)
    );
    assert_eq!(
        labels.resolve_slot_label(
            "default",
            &crate::ui_components::color_dot::TAB_COLOR_OPTIONS
        ),
        Some(TabColorSlot::Default)
    );
    assert_eq!(
        labels.resolve_slot_label("none", &crate::ui_components::color_dot::TAB_COLOR_OPTIONS),
        Some(TabColorSlot::Default)
    );
}

#[test]
fn color_slot_labels_reject_duplicate_labels() {
    let labels = TabColorSlotLabels::default()
        .with_label(
            TabColorSlot::Default,
            "Inactive",
            &crate::ui_components::color_dot::TAB_COLOR_OPTIONS,
        )
        .expect("default label should save");

    assert_eq!(
        labels.with_label(
            AnsiColorIdentifier::Blue,
            "inactive",
            &crate::ui_components::color_dot::TAB_COLOR_OPTIONS,
        ),
        Err(TabColorSlotLabelError::DuplicateLabel)
    );
}

#[test]
fn color_slot_labels_reject_reserved_labels() {
    let labels = TabColorSlotLabels::default();

    for label in ["default", "none", "blue", "RED", "black", "WHITE"] {
        assert_eq!(
            labels.with_label(
                TabColorSlot::Default,
                label,
                &crate::ui_components::color_dot::TAB_COLOR_OPTIONS,
            ),
            Err(TabColorSlotLabelError::ReservedLabel),
            "{label} should be reserved"
        );
    }
}

#[test]
fn color_slot_labels_normalize_persisted_values() {
    let value = serde_json::json!({
        "default": "  Inactive  ",
        "black": "Dark",
        "red": "black",
        "green": "",
        "blue": "GOAL: primary",
        "magenta": "goal: PRIMARY",
        "white": "Light",
    });

    let labels: TabColorSlotLabels =
        serde_json::from_value(value.clone()).expect("persisted labels should deserialize");
    assert_eq!(
        labels.custom_label_for_slot(TabColorSlot::Default),
        Some("Inactive")
    );
    assert_eq!(
        labels.custom_label_for_slot(TabColorSlot::Blue),
        Some("GOAL: primary")
    );
    assert_eq!(labels.custom_label_for_slot(TabColorSlot::Black), None);
    assert_eq!(labels.custom_label_for_slot(TabColorSlot::Red), None);
    assert_eq!(labels.custom_label_for_slot(TabColorSlot::Green), None);
    assert_eq!(labels.custom_label_for_slot(TabColorSlot::Magenta), None);
    assert_eq!(labels.custom_label_for_slot(TabColorSlot::White), None);
    assert_eq!(
        labels.resolve_label("black", &crate::ui_components::color_dot::TAB_COLOR_OPTIONS),
        None
    );

    let file_labels =
        <TabColorSlotLabels as settings_value::SettingsValue>::from_file_value(&value)
            .expect("settings file labels should deserialize");
    assert_eq!(file_labels, labels);
}

#[test]
fn color_slot_labels_clear_back_to_default_name() {
    let labels = TabColorSlotLabels::default()
        .with_label(
            AnsiColorIdentifier::Blue,
            "GOAL: primary",
            &crate::ui_components::color_dot::TAB_COLOR_OPTIONS,
        )
        .expect("blue label should save")
        .without_label(AnsiColorIdentifier::Blue);

    assert_eq!(labels.label_for(AnsiColorIdentifier::Blue), "Blue");
}

#[test]
fn use_latest_user_prompt_as_conversation_title_in_tab_names_defaults_to_false() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);

        TabSettings::handle(&app).read(&app, |settings, _ctx| {
            assert!(!*settings.use_latest_user_prompt_as_conversation_title_in_tab_names);
        });
    });
}

#[test]
fn use_latest_user_prompt_as_conversation_title_in_tab_names_uses_vertical_tabs_path() {
    assert_eq!(
        UseLatestUserPromptAsConversationTitleInTabNames::toml_path(),
        Some("appearance.vertical_tabs.use_latest_prompt_as_title")
    );
    assert_eq!(
        UseLatestUserPromptAsConversationTitleInTabNames::hierarchy(),
        Some("appearance.vertical_tabs")
    );
    assert_eq!(
        UseLatestUserPromptAsConversationTitleInTabNames::toml_key(),
        "use_latest_prompt_as_title"
    );
}

#[test]
fn show_vertical_tab_panel_in_restored_windows_defaults_to_false() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);

        TabSettings::handle(&app).read(&app, |settings, _ctx| {
            assert!(!*settings.show_vertical_tab_panel_in_restored_windows);
        });
    });
}

#[test]
fn show_vertical_tab_panel_in_restored_windows_uses_vertical_tabs_path() {
    assert_eq!(
        ShowVerticalTabPanelInRestoredWindows::toml_path(),
        Some("appearance.vertical_tabs.show_panel_in_restored_windows")
    );
    assert_eq!(
        ShowVerticalTabPanelInRestoredWindows::hierarchy(),
        Some("appearance.vertical_tabs")
    );
    assert_eq!(
        ShowVerticalTabPanelInRestoredWindows::toml_key(),
        "show_panel_in_restored_windows"
    );
}

#[test]
fn hide_title_bar_search_bar_in_vertical_tabs_defaults_to_false() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);

        TabSettings::handle(&app).read(&app, |settings, _ctx| {
            assert!(!*settings.hide_title_bar_search_bar_in_vertical_tabs);
        });
    });
}

#[test]
fn hide_title_bar_search_bar_in_vertical_tabs_uses_vertical_tabs_path() {
    assert_eq!(
        HideTitleBarSearchBarInVerticalTabs::toml_path(),
        Some("appearance.vertical_tabs.hide_title_bar_search_bar")
    );
    assert_eq!(
        HideTitleBarSearchBarInVerticalTabs::hierarchy(),
        Some("appearance.vertical_tabs")
    );
    assert_eq!(
        HideTitleBarSearchBarInVerticalTabs::toml_key(),
        "hide_title_bar_search_bar"
    );
}

#[test]
fn header_toolbar_chip_selection_default_contains_code_review() {
    let config = HeaderToolbarChipSelection::Default;
    assert!(config.contains_item(&HeaderToolbarItemKind::CodeReview));
}

#[test]
fn header_toolbar_chip_selection_custom_without_code_review_reports_absent() {
    let config = HeaderToolbarChipSelection::Custom {
        left: vec![
            HeaderToolbarItemKind::TabsPanel,
            HeaderToolbarItemKind::ToolsPanel,
        ],
        right: vec![HeaderToolbarItemKind::NotificationsMailbox],
    };
    assert!(!config.contains_item(&HeaderToolbarItemKind::CodeReview));
    assert!(config.contains_item(&HeaderToolbarItemKind::TabsPanel));
    assert!(config.contains_item(&HeaderToolbarItemKind::ToolsPanel));
    assert!(config.contains_item(&HeaderToolbarItemKind::NotificationsMailbox));
    assert!(!config.contains_item(&HeaderToolbarItemKind::AgentManagement));
}

#[test]
fn header_toolbar_chip_selection_custom_with_code_review_on_left_reports_present() {
    let config = HeaderToolbarChipSelection::Custom {
        left: vec![HeaderToolbarItemKind::CodeReview],
        right: vec![],
    };
    assert!(config.contains_item(&HeaderToolbarItemKind::CodeReview));
}

#[test]
fn header_toolbar_chip_selection_custom_empty_reports_all_absent() {
    let config = HeaderToolbarChipSelection::Custom {
        left: vec![],
        right: vec![],
    };
    for item in HeaderToolbarItemKind::all_items() {
        assert!(!config.contains_item(&item));
    }
}
