use warpui::assets::asset_cache::AssetSource;
use warpui::elements::{
    Align, CacheOption, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, Image,
    MainAxisAlignment, MouseStateHandle, ParentElement, Wrap,
};
use warpui::ui_components::components::UiComponent;
use warpui::{AppContext, Entity, View, ViewContext, ViewHandle};

use super::settings_page::{
    MatchData, PageType, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle,
    SettingsWidget,
};
use super::SettingsSection;
use crate::appearance::Appearance;
use crate::channel::ChannelState;
use crate::themes::theme::ColorScheme;
use crate::workspace::WorkspaceAction;

pub struct AboutPageView {
    page: PageType<Self>,
}

impl AboutPageView {
    pub fn new(_ctx: &mut ViewContext<AboutPageView>) -> Self {
        AboutPageView {
            page: PageType::new_monolith(AboutPageWidget::default(), None, false),
        }
    }
}

impl Entity for AboutPageView {
    type Event = SettingsPageEvent;
}

impl View for AboutPageView {
    fn ui_name() -> &'static str {
        "AboutPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Default)]
struct AboutPageWidget {
    copy_version_button_mouse_state: MouseStateHandle,
    copy_rbf_version_button_mouse_state: MouseStateHandle,
}

impl SettingsWidget for AboutPageWidget {
    type View = AboutPageView;

    fn search_terms(&self) -> &str {
        "about warp version"
    }

    fn render(
        &self,
        _view: &AboutPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let ui_builder = appearance.ui_builder();

        let image_path = if theme.inferred_color_scheme() == ColorScheme::LightOnDark {
            "bundled/svg/warp-logo-with-light-title.svg"
        } else {
            "bundled/svg/warp-logo-with-dark-title.svg"
        };

        let rbf_version = include_str!("../../../warp-rbf/RBF_VERSION").trim();
        let rbf_version_text = format!("Warp RBF v{rbf_version}");
        let (warp_version_label, warp_version_copy_text) = warp_version_text();

        let version_row = render_version_row(
            warp_version_label,
            warp_version_copy_text,
            self.copy_version_button_mouse_state.clone(),
            appearance,
        );
        let rbf_version_row = render_version_row(
            rbf_version_text.clone(),
            rbf_version_text,
            self.copy_rbf_version_button_mouse_state.clone(),
            appearance,
        );

        Align::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    ConstrainedBox::new(
                        Image::new(
                            AssetSource::Bundled { path: image_path },
                            CacheOption::BySize,
                        )
                        .finish(),
                    )
                    .with_max_height(100.)
                    .with_max_width(350.)
                    .finish(),
                )
                .with_child(version_row.finish())
                .with_child(rbf_version_row.finish())
                .with_child(
                    ui_builder
                        .span("Copyright 2026 Warp")
                        .build()
                        .with_margin_top(16.)
                        .finish(),
                )
                .finish(),
        )
        .finish()
    }
}

fn warp_version_text() -> (String, String) {
    if let Some(version) = ChannelState::app_version() {
        return (format!("Warp {version}"), version.to_string());
    }

    if let Some(source_sha) = option_env!("WARP_BUILD_SOURCE_SHA").filter(|sha| !sha.is_empty()) {
        let short_sha = &source_sha[..source_sha.len().min(12)];
        return (
            format!("Warp source {short_sha}"),
            format!("Warp source {source_sha}"),
        );
    }

    (
        "Warp source unknown".to_string(),
        "Warp source unknown".to_string(),
    )
}

fn render_version_row(
    label: String,
    copy_text: String,
    copy_button_mouse_state: MouseStateHandle,
    appearance: &Appearance,
) -> Wrap {
    let ui_builder = appearance.ui_builder();

    let version_text = ui_builder
        .span(label)
        .with_soft_wrap()
        .build()
        .with_margin_top(16.)
        .finish();

    let copy_version_icon = ui_builder
        .copy_button(16., copy_button_mouse_state)
        .build()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(WorkspaceAction::CopyVersion(copy_text.clone()));
        })
        .finish();

    Wrap::row()
        .with_main_axis_alignment(MainAxisAlignment::Center)
        .with_children([
            version_text,
            Container::new(copy_version_icon)
                .with_margin_top(16.)
                .with_padding_left(6.)
                .finish(),
        ])
}

impl SettingsPageMeta for AboutPageView {
    fn section() -> SettingsSection {
        SettingsSection::About
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<AboutPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AboutPageView>) -> Self {
        SettingsPageViewHandle::About(view_handle)
    }
}
