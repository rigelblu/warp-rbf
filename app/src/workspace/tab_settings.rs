use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use settings::macros::define_settings_group;
use settings::{RespectUserSyncSetting, SupportedPlatforms, SyncToCloud};
use warp_core::ui::theme::AnsiColorIdentifier;

pub(crate) const TAB_COLOR_OPTIONS: [AnsiColorIdentifier; 6] = [
    AnsiColorIdentifier::Red,
    AnsiColorIdentifier::Green,
    AnsiColorIdentifier::Yellow,
    AnsiColorIdentifier::Blue,
    AnsiColorIdentifier::Magenta,
    AnsiColorIdentifier::Cyan,
];

#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Where new tabs are placed in the tab bar.",
    rename_all = "snake_case"
)]
pub enum NewTabPlacement {
    #[default]
    AfterCurrentTab,
    AfterAllTabs,
}

settings::macros::implement_setting_for_enum!(
    NewTabPlacement,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Never,
    private: false,
    toml_path: "general.new_tab_placement",
    description: "Where new tabs are placed in the tab bar.",
);

#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Position of the close button on tabs.",
    rename_all = "snake_case"
)]
pub enum TabCloseButtonPosition {
    #[default]
    Right,
    Left,
}

settings::macros::implement_setting_for_enum!(
    TabCloseButtonPosition,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.tabs.tab_close_button_position",
    description: "Position of the close button on tabs.",
);

/// Visibility options for workspace decorations like the tab bar.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "When workspace decorations such as the tab bar are visible.",
    rename_all = "snake_case"
)]
pub enum WorkspaceDecorationVisibility {
    /// Always show workspace decorations.
    AlwaysShow,
    /// Hide workspace decorations if fullscreen.
    #[default]
    HideFullscreen,
    /// Only show workspace decorations on hover.
    OnHover,
}

settings::macros::implement_setting_for_enum!(
    WorkspaceDecorationVisibility,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.tabs.workspace_decoration_visibility",
    description: "When workspace decorations such as the tab bar are visible.",
);

impl WorkspaceDecorationVisibility {
    /// Choose a visibility setting that's logically opposite from this one.
    pub fn toggled(self) -> Self {
        // If we add other variants, there should still be logical opposites for each. For example,
        // toggling from any form of hidden workspace decorations should re-enable them.
        match self {
            WorkspaceDecorationVisibility::AlwaysShow => WorkspaceDecorationVisibility::OnHover,
            WorkspaceDecorationVisibility::OnHover => WorkspaceDecorationVisibility::HideFullscreen,
            WorkspaceDecorationVisibility::HideFullscreen => WorkspaceDecorationVisibility::OnHover,
        }
    }

    /// True if this is a setting where workspace decorations are hidden by default.
    pub fn hides_decorations_by_default(self) -> bool {
        matches!(self, WorkspaceDecorationVisibility::OnHover,)
    }

    /// True if *window* decorations should be shown.
    pub fn show_window_decorations(self) -> bool {
        !matches!(self, WorkspaceDecorationVisibility::OnHover)
    }
}

/// Represents the color state for a directory entry in the tab-color settings.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Color assignment state for a directory's tab.",
    rename_all = "snake_case"
)]
pub enum DirectoryTabColor {
    /// User explicitly removed this directory. Retained for backwards compatibility with settings files written by older versions.
    #[schemars(description = "The directory was explicitly removed from tab coloring.")]
    Suppressed,
    /// Directory is tracked but has no assigned color.
    #[schemars(description = "The directory is tracked but has no assigned color.")]
    Unassigned,
    /// Directory is tracked with a specific color.
    #[schemars(description = "The directory is assigned a specific color.")]
    Color(AnsiColorIdentifier),
}

impl DirectoryTabColor {
    pub(crate) fn ansi_color(self) -> Option<AnsiColorIdentifier> {
        match self {
            DirectoryTabColor::Color(c) => Some(c),
            DirectoryTabColor::Suppressed | DirectoryTabColor::Unassigned => None,
        }
    }
}

/// User-configured directory→color mappings for tab coloring.
///
/// Keys are directory paths (as strings). Values indicate the color state:
/// - `Suppressed`: directory was explicitly removed by the user via the per-row X button.
///   Retained so `color_for_directory` can shadow broader prefix matches, and for
///   backwards compatibility with settings files written by older versions.
/// - `Unassigned`: directory is tracked but has no specific color.
/// - `Color(c)`: directory is tracked with the given color.
#[derive(
    Default,
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(description = "Mapping of directory paths to their tab color assignments.")]
pub struct DirectoryTabColors(pub(crate) HashMap<String, DirectoryTabColor>);

settings::macros::implement_setting_for_enum!(
    DirectoryTabColors,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Never,
    private: false,
    toml_path: "appearance.tabs.directory_tab_colors",
    max_table_depth: 0,
    description: "Mapping of directory paths to their tab color assignments.",
    feature_flag: warp_core::features::FeatureFlag::DirectoryTabColors,
);

impl DirectoryTabColors {
    /// Returns the configured tab color for a directory using longest-prefix matching.
    /// Returns `None` if no configured directory is a prefix of `dir`.
    pub fn color_for_directory(&self, canonical_dir: &Path) -> Option<DirectoryTabColor> {
        self.0
            .iter()
            .filter_map(|(configured_path, color)| {
                let configured = Path::new(configured_path);
                match color {
                    DirectoryTabColor::Suppressed => None,
                    _ => canonical_dir
                        .starts_with(configured)
                        .then_some((configured, *color)),
                }
            })
            .max_by_key(|(configured, _)| configured.as_os_str().len())
            .map(|(_, color)| color)
    }

    /// Returns a new value with the given directory's color updated.
    pub fn with_color(&self, path: &Path, color: DirectoryTabColor) -> Self {
        let mut map = self.0.clone();
        map.insert(canonical_directory_key(path), color);
        Self(map)
    }
}

/// A fixed tab color slot that can be assigned a user-facing label.
#[derive(
    Debug,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    Hash,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "A fixed tab color slot.")]
pub enum TabColorSlot {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl TabColorSlot {
    pub fn raw_label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Black => "Black",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Yellow => "Yellow",
            Self::Blue => "Blue",
            Self::Magenta => "Magenta",
            Self::Cyan => "Cyan",
            Self::White => "White",
        }
    }

    pub fn color(self) -> Option<AnsiColorIdentifier> {
        match self {
            Self::Default => None,
            Self::Black => Some(AnsiColorIdentifier::Black),
            Self::Red => Some(AnsiColorIdentifier::Red),
            Self::Green => Some(AnsiColorIdentifier::Green),
            Self::Yellow => Some(AnsiColorIdentifier::Yellow),
            Self::Blue => Some(AnsiColorIdentifier::Blue),
            Self::Magenta => Some(AnsiColorIdentifier::Magenta),
            Self::Cyan => Some(AnsiColorIdentifier::Cyan),
            Self::White => Some(AnsiColorIdentifier::White),
        }
    }

    pub fn parse(input: &str, allowed_colors: &[AnsiColorIdentifier]) -> Option<Self> {
        let input = input.trim();
        if input.eq_ignore_ascii_case("default") || input.eq_ignore_ascii_case("none") {
            return Some(Self::Default);
        }

        input
            .parse::<AnsiColorIdentifier>()
            .ok()
            .filter(|color| allowed_colors.contains(color))
            .map(Self::from)
    }
}

impl std::fmt::Display for TabColorSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.raw_label())
    }
}

impl From<AnsiColorIdentifier> for TabColorSlot {
    fn from(color: AnsiColorIdentifier) -> Self {
        match color {
            AnsiColorIdentifier::Black => Self::Black,
            AnsiColorIdentifier::Red => Self::Red,
            AnsiColorIdentifier::Green => Self::Green,
            AnsiColorIdentifier::Yellow => Self::Yellow,
            AnsiColorIdentifier::Blue => Self::Blue,
            AnsiColorIdentifier::Magenta => Self::Magenta,
            AnsiColorIdentifier::Cyan => Self::Cyan,
            AnsiColorIdentifier::White => Self::White,
        }
    }
}

/// User-configured labels for the fixed tab color slots.
#[derive(Default, Debug, Clone, PartialEq, Eq, schemars::JsonSchema)]
#[schemars(description = "Custom display labels for tab color slots.")]
pub struct TabColorSlotLabels(HashMap<TabColorSlot, String>);

impl Serialize for TabColorSlotLabels {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.normalized().0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TabColorSlotLabels {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(HashMap::deserialize(deserializer)?).normalized())
    }
}

impl settings_value::SettingsValue for TabColorSlotLabels {
    fn to_file_value(&self) -> serde_json::Value {
        <HashMap<TabColorSlot, String> as settings_value::SettingsValue>::to_file_value(
            &self.normalized().0,
        )
    }

    fn from_file_value(value: &serde_json::Value) -> Option<Self> {
        Some(
            Self(
                <HashMap<TabColorSlot, String> as settings_value::SettingsValue>::from_file_value(
                    value,
                )?,
            )
            .normalized(),
        )
    }
}

settings::macros::implement_setting_for_enum!(
    TabColorSlotLabels,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.tabs.color_slot_labels",
    max_table_depth: 0,
    description: "Custom display labels for tab color slots.",
);

impl TabColorSlotLabels {
    pub fn custom_label_for_slot(&self, slot: TabColorSlot) -> Option<&str> {
        self.0.get(&slot).map(String::as_str)
    }

    pub fn custom_label(&self, color: AnsiColorIdentifier) -> Option<&str> {
        self.custom_label_for_slot(color.into())
    }

    pub fn label_for_slot(&self, slot: TabColorSlot) -> String {
        self.custom_label_for_slot(slot)
            .filter(|label| !label.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| slot.raw_label().to_owned())
    }

    pub fn label_for(&self, color: AnsiColorIdentifier) -> String {
        self.label_for_slot(color.into())
    }

    pub fn display_label_for_slot(&self, slot: TabColorSlot) -> String {
        match self
            .custom_label_for_slot(slot)
            .filter(|label| !label.trim().is_empty())
        {
            Some(label) => format!("{} ({})", label.trim(), slot.raw_label()),
            None => slot.raw_label().to_owned(),
        }
    }

    pub fn display_label_for(&self, color: AnsiColorIdentifier) -> String {
        self.display_label_for_slot(color.into())
    }

    pub fn resolve_slot_label(
        &self,
        input: &str,
        allowed_colors: &[AnsiColorIdentifier],
    ) -> Option<TabColorSlot> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        let labels = self.normalized_with_allowed_colors(allowed_colors);
        TabColorSlot::parse(input, allowed_colors).or_else(|| {
            tab_color_slots(allowed_colors).find(|slot| {
                labels
                    .custom_label_for_slot(*slot)
                    .is_some_and(|label| label.trim().eq_ignore_ascii_case(input))
            })
        })
    }

    pub fn resolve_label(
        &self,
        input: &str,
        allowed_colors: &[AnsiColorIdentifier],
    ) -> Option<AnsiColorIdentifier> {
        self.resolve_slot_label(input, allowed_colors)
            .and_then(TabColorSlot::color)
    }

    fn is_reserved_label(label: &str) -> bool {
        label.eq_ignore_ascii_case("default")
            || label.eq_ignore_ascii_case("none")
            || label.parse::<AnsiColorIdentifier>().is_ok()
    }

    pub fn normalized(&self) -> Self {
        self.normalized_with_allowed_colors(&TAB_COLOR_OPTIONS)
    }

    fn normalized_with_allowed_colors(&self, allowed_colors: &[AnsiColorIdentifier]) -> Self {
        let mut labels = HashMap::new();
        for slot in tab_color_slots(allowed_colors) {
            let Some(label) = self.0.get(&slot).map(|label| label.trim()) else {
                continue;
            };
            if label.is_empty() || Self::is_reserved_label(label) {
                continue;
            }
            if labels
                .values()
                .any(|existing_label: &String| existing_label.trim().eq_ignore_ascii_case(label))
            {
                continue;
            }
            labels.insert(slot, label.to_owned());
        }
        Self(labels)
    }

    pub fn with_label(
        &self,
        slot: impl Into<TabColorSlot>,
        label: impl Into<String>,
        allowed_colors: &[AnsiColorIdentifier],
    ) -> Result<Self, TabColorSlotLabelError> {
        let slot = slot.into();
        if let Some(color) = slot.color() {
            if !allowed_colors.contains(&color) {
                return Err(TabColorSlotLabelError::UnknownColor);
            }
        }

        let label = label.into();
        let label = label.trim();
        if label.is_empty() {
            return Err(TabColorSlotLabelError::EmptyLabel);
        }

        if Self::is_reserved_label(label) {
            return Err(TabColorSlotLabelError::ReservedLabel);
        }

        let current_labels = self.normalized_with_allowed_colors(allowed_colors);
        if current_labels
            .0
            .iter()
            .any(|(existing_slot, existing_label)| {
                *existing_slot != slot && existing_label.trim().eq_ignore_ascii_case(label)
            })
        {
            return Err(TabColorSlotLabelError::DuplicateLabel);
        }

        let mut labels = current_labels.0;
        labels.insert(slot, label.to_owned());
        Ok(Self(labels))
    }

    pub fn without_label(&self, slot: impl Into<TabColorSlot>) -> Self {
        let mut labels = self.normalized().0;
        labels.remove(&slot.into());
        Self(labels)
    }
}

fn tab_color_slots(
    allowed_colors: &[AnsiColorIdentifier],
) -> impl Iterator<Item = TabColorSlot> + '_ {
    std::iter::once(TabColorSlot::Default)
        .chain(allowed_colors.iter().copied().map(TabColorSlot::from))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabColorSlotLabelError {
    UnknownColor,
    EmptyLabel,
    ReservedLabel,
    DuplicateLabel,
}

/// Canonicalizes `path` into the string key used in [`DirectoryTabColors`].
pub fn canonical_directory_key(path: &Path) -> String {
    dunce::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

#[derive(
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Configuration for the header toolbar chips in the vertical tab panel header.",
    rename_all = "snake_case"
)]
pub enum HeaderToolbarChipSelection {
    #[default]
    Default,
    Custom {
        left: Vec<super::header_toolbar_item::HeaderToolbarItemKind>,
        right: Vec<super::header_toolbar_item::HeaderToolbarItemKind>,
    },
}

impl HeaderToolbarChipSelection {
    pub fn left_items(&self) -> Vec<super::header_toolbar_item::HeaderToolbarItemKind> {
        use super::header_toolbar_item::HeaderToolbarItemKind;
        match self {
            Self::Default => HeaderToolbarItemKind::default_left(),
            Self::Custom { left, .. } => left.clone(),
        }
    }

    pub fn right_items(&self) -> Vec<super::header_toolbar_item::HeaderToolbarItemKind> {
        use super::header_toolbar_item::HeaderToolbarItemKind;
        match self {
            Self::Default => HeaderToolbarItemKind::default_right(),
            Self::Custom { right, .. } => right.clone(),
        }
    }

    pub fn contains_item(&self, item: &super::header_toolbar_item::HeaderToolbarItemKind) -> bool {
        self.left_items().contains(item) || self.right_items().contains(item)
    }
}

settings::macros::implement_setting_for_enum!(
    HeaderToolbarChipSelection,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.tabs.header_toolbar_chip_selection",
    description: "Configuration for the header toolbar chips in the vertical tab panel header.",
);

#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Display mode for the vertical tab bar.",
    rename_all = "snake_case"
)]
pub enum VerticalTabsViewMode {
    #[default]
    Compact,
    Expanded,
}

settings::macros::implement_setting_for_enum!(
    VerticalTabsViewMode,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.vertical_tabs.view_mode",
    description: "Display mode for the vertical tab bar.",
);

#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Granularity of rows displayed in the vertical tabs panel.",
    rename_all = "snake_case"
)]
pub enum VerticalTabsDisplayGranularity {
    #[default]
    Panes,
    Tabs,
}

settings::macros::implement_setting_for_enum!(
    VerticalTabsDisplayGranularity,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.vertical_tabs.display_granularity",
    description: "Granularity of rows displayed in the vertical tabs panel.",
);

#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Tab item display mode in vertical tabs.",
    rename_all = "snake_case"
)]
pub enum VerticalTabsTabItemMode {
    #[default]
    FocusedSession,
    Summary,
}

settings::macros::implement_setting_for_enum!(
    VerticalTabsTabItemMode,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.vertical_tabs.tab_item_mode",
    description: "Tab item display mode in vertical tabs.",
);

#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Primary information displayed on vertical tabs.",
    rename_all = "snake_case"
)]
pub enum VerticalTabsPrimaryInfo {
    #[default]
    Command,
    WorkingDirectory,
    Branch,
}

settings::macros::implement_setting_for_enum!(
    VerticalTabsPrimaryInfo,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.vertical_tabs.primary_info",
    description: "The primary information displayed on vertical tabs.",
);

#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Subtitle shown on compact vertical tabs.",
    rename_all = "snake_case"
)]
pub enum VerticalTabsCompactSubtitle {
    #[default]
    Branch,
    WorkingDirectory,
    Command,
}

settings::macros::implement_setting_for_enum!(
    VerticalTabsCompactSubtitle,
    TabSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "appearance.vertical_tabs.compact_subtitle",
    description: "Subtitle shown on compact vertical tabs.",
);

define_settings_group!(TabSettings, settings: [
    show_indicators: ShowIndicatorsButton {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.tabs.show_indicators_button",
        description: "Whether to show activity indicators on tabs.",
    },
    show_code_review_button: ShowCodeReviewButton {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "code.editor.show_code_review_button",
        description: "Whether to show the code review button on tabs.",
    },
    show_code_review_diff_stats: ShowCodeReviewDiffStats {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "code.editor.show_code_review_diff_stats",
        description: "Whether to show lines added/removed counts on the code review button.",
    },
    preserve_active_tab_color: PreserveActiveTabColor {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.tabs.preserve_active_tab_color",
        description: "Whether to preserve the active tab's color when switching tabs.",
    },
    use_vertical_tabs: UseVerticalTabs {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.vertical_tabs.enabled",
        description: "Whether to display tabs vertically instead of horizontally.",
    },
    show_vertical_tab_panel_in_restored_windows: ShowVerticalTabPanelInRestoredWindows {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.vertical_tabs.show_panel_in_restored_windows",
        description: "When restoring a window, open the vertical tabs panel even if it was closed when the session was saved.",
    },
    hide_title_bar_search_bar_in_vertical_tabs: HideTitleBarSearchBarInVerticalTabs {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.vertical_tabs.hide_title_bar_search_bar",
        description: "When using the vertical tab layout, hide the search bar in the title bar. Search stays available via the command palette and keyboard shortcuts.",
    },
    use_latest_user_prompt_as_conversation_title_in_tab_names: UseLatestUserPromptAsConversationTitleInTabNames {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.vertical_tabs.use_latest_prompt_as_title",
        description: "Whether vertical tab names for agent conversations use the latest user prompt.",
    },
    vertical_tabs_display_granularity: VerticalTabsDisplayGranularity,
    vertical_tabs_tab_item_mode: VerticalTabsTabItemMode,
    vertical_tabs_view_mode: VerticalTabsViewMode,
    vertical_tabs_primary_info: VerticalTabsPrimaryInfo,
    vertical_tabs_compact_subtitle: VerticalTabsCompactSubtitle,
    vertical_tabs_show_pr_link: VerticalTabsShowPrLink {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.vertical_tabs.show_pr_link",
        description: "Whether to show PR links on vertical tabs.",
    },
    vertical_tabs_show_diff_stats: VerticalTabsShowDiffStats {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.vertical_tabs.show_diff_stats",
        description: "Whether to show diff stats on vertical tabs.",
    },
    vertical_tabs_show_details_on_hover: VerticalTabsShowDetailsOnHover {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "appearance.vertical_tabs.show_details_on_hover",
        description: "Whether to show a details sidecar when hovering over a vertical tab.",
    },
    header_toolbar_chip_selection: HeaderToolbarChipSelection,
    new_tab_placement: NewTabPlacement,
    workspace_decoration_visibility: WorkspaceDecorationVisibility,
    close_button_position: TabCloseButtonPosition,
    directory_tab_colors: DirectoryTabColors,
    color_slot_labels: TabColorSlotLabels,
]);

#[cfg(test)]
#[path = "tab_settings_tests.rs"]
mod tests;
