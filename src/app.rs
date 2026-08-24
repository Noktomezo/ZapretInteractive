use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::*;

use crate::app_state::AppState;
use crate::faulty_terminal::FaultyTerminal;
use crate::ui::components::button::Button;
use crate::ui::components::dropdown::{DropdownChoice, DropdownEvent, DropdownState};
use crate::ui::components::text_input::{TextInputEvent, TextInputState};
use crate::ui::foundation::colors::{
    accent_foreground, background, border, card as card_color, destructive, foreground, green,
    muted_foreground, paper, secondary, yellow as accent,
};
use crate::ui::foundation::control_style::{SHELL_CONTROL_SIZE, SHELL_SPACING};
use crate::ui::foundation::motion::{DropdownMotion, ScalarTransition, mix_color};

mod setup;

const SIDEBAR_MOTION: Duration = Duration::from_millis(250);
const SIDEBAR_EXPANDED_WIDTH: f32 = 182.4;
const SIDEBAR_COLLAPSED_WIDTH: f32 = 40.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Home,
    Modules,
    Dns,
    TgProxy,
    Strategies,
    StrategyProbe,
    Category(String),
    Filters,
    Placeholders,
    Logs,
    Settings,
    About,
}

#[derive(Clone, Debug)]
pub enum EditorTarget {
    Category(Option<String>),
    Strategy {
        category_id: String,
        strategy_id: Option<String>,
    },
    Placeholder(Option<usize>),
    Filter(Option<String>),
}

const ACRYLIC_MOTION: Duration = Duration::from_millis(250);

fn sidebar_motion(collapsed: bool) -> ScalarTransition {
    ScalarTransition::new(f32::from(!collapsed), SIDEBAR_MOTION)
}

fn acrylic_motion(enabled: bool) -> ScalarTransition {
    ScalarTransition::new(f32::from(enabled), ACRYLIC_MOTION)
}

pub struct AppView {
    pub state: Entity<AppState>,
    pub route: Route,
    pub editor: Option<EditorTarget>,
    pub primary_input: Entity<TextInputState>,
    pub secondary_input: Entity<TextInputState>,
    pub text_area: Entity<crate::ui::components::text_area::TextAreaState>,
    pub tcp_input: Entity<TextInputState>,
    pub udp_input: Entity<TextInputState>,
    pub tg_port_input: Entity<TextInputState>,
    pub tg_secret_input: Entity<TextInputState>,
    pub theme_dropdown: Entity<DropdownState>,
    pub language_dropdown: Entity<DropdownState>,
    pub discord_dropdown: Entity<DropdownState>,
    pub dns_dropdown: Entity<DropdownState>,
    pub(crate) faulty_terminal: Entity<FaultyTerminal>,
    sidebar_collapsed: bool,
    pub(crate) tg_info_expanded: bool,
    sidebar_motion: ScalarTransition,
    acrylic_motion: ScalarTransition,
    page_revision: u64,
    _subscriptions: Vec<Subscription>,
    pub(crate) selected_at: Instant,
    pub(crate) deselected_route: Option<Route>,
    pub(crate) deselected_at: Option<Instant>,
    pub hovered_route: Option<Route>,
    pub(crate) unhovered_route: Option<Route>,
    pub(crate) hovered_at: Option<Instant>,
    pub(crate) unhovered_at: Option<Instant>,
    pub categories_scroll_handle: UniformListScrollHandle,
    pub filters_scroll_handle: UniformListScrollHandle,
    pub placeholders_scroll_handle: UniformListScrollHandle,
    pub category_strategies_list_state: ListState,
    pub(crate) current_viewed_category: Option<String>,
    pub(crate) closing_editor: Option<(EditorTarget, Instant)>,
    pub(crate) closing_confirm: Option<(
        crate::ui::components::confirm_dialog::ConfirmTarget,
        Instant,
    )>,
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (progress, animating) = self.sidebar_motion.sample();
        let (acrylic_progress, acrylic_animating) = self.acrylic_motion.sample();
        if animating || acrylic_animating {
            cx.on_next_frame(window, |_, _, cx| cx.notify());
        }
        let state = self.state.clone();
        let toggle_sidebar = cx.listener(move |this, _, _, cx| {
            this.sidebar_collapsed = !this.sidebar_collapsed;
            this.sidebar_motion
                .set_target(f32::from(!this.sidebar_collapsed));
            let collapsed = this.sidebar_collapsed;
            state.update(cx, |s, cx| s.set_sidebar_collapsed(collapsed, cx));
            cx.notify();
        });
        let terminal = self.faulty_terminal.clone();
        let status = self.state.read(cx).status;
        self.faulty_terminal.update(cx, |terminal, _| {
            terminal.set_status(status);
            terminal.set_dark_theme(crate::ui::foundation::colors::is_dark());
        });
        let is_home = matches!(self.route, Route::Home);
        let breadcrumb = self.breadcrumb(cx);
        let page = page_transition(self.render_page(cx), self.page_revision);
        let shell_opacity = (1.0 - 0.50 * acrylic_progress).clamp(0.50, 1.0);

        let editor_closing_progress = if self.editor.is_some() {
            Some(None)
        } else if let Some((_, started_at)) = self.closing_editor.as_ref() {
            let elapsed = started_at.elapsed().as_secs_f32();
            let duration = crate::ui::foundation::motion::DIALOG_MOTION.as_secs_f32();
            if elapsed < duration {
                window.request_animation_frame();
                let progress = 1.0 - (elapsed / duration).clamp(0.0, 1.0);
                Some(Some(progress))
            } else {
                None
            }
        } else {
            None
        };

        let confirm_target = self.state.read(cx).confirm.clone();
        let confirm_closing_progress = if let Some(target) = confirm_target {
            Some((target, None))
        } else if let Some((target, started_at)) = self.closing_confirm.as_ref() {
            let elapsed = started_at.elapsed().as_secs_f32();
            let duration = crate::ui::foundation::motion::DIALOG_MOTION.as_secs_f32();
            if elapsed < duration {
                window.request_animation_frame();
                let progress = 1.0 - (elapsed / duration).clamp(0.0, 1.0);
                Some((target.clone(), Some(progress)))
            } else {
                None
            }
        } else {
            None
        };

        let root_handle = cx.entity().downgrade();
        let on_cancel = {
            let root = root_handle.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                if let Some(root) = root.upgrade() {
                    root.update(cx, |this, cx| {
                        if let Some(target) = this.state.read(cx).confirm.clone() {
                            this.closing_confirm = Some((target, Instant::now()));
                            this.state.update(cx, |s, cx| s.set_confirm(None, cx));
                        }
                    });
                }
            }
        };
        let on_confirm = {
            let root = root_handle.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                if let Some(root) = root.upgrade() {
                    root.update(cx, |this, cx| {
                        if let Some(target) = this.state.read(cx).confirm.clone() {
                            this.closing_confirm = Some((target, Instant::now()));
                            this.state.update(cx, |s, cx| s.execute_confirm(cx));
                        }
                    });
                }
            }
        };

        div()
            .size_full()
            .font_family("IBM Plex Sans")
            .bg(card_color().opacity(shell_opacity))
            .text_color(foreground())
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(move |_, event: &MouseMoveEvent, window, cx| {
                terminal.update(cx, |terminal, _| terminal.set_mouse(event.position));
                if crate::ui::components::cursor_tooltip::update_position(event.position, cx) {
                    window.refresh();
                }
                if crate::pages::update_category_drag_mouse(event.position, cx) {
                    cx.notify();
                    window.refresh();
                }
            }))
            .child(titlebar(
                window.is_maximized(),
                progress,
                acrylic_progress,
                breadcrumb,
                self.state.read(cx).config.minimize_to_tray,
                self.state.clone(),
                toggle_sidebar,
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .relative()
                    .overflow_hidden()
                    .child(self.sidebar(progress, acrylic_progress, window, cx))
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .when(!is_home, |main| main.bg(background()))
                            .border_t(px(1.))
                            .border_l(px(1.))
                            .border_color(border())
                            .rounded_tl(px(8.))
                            .overflow_hidden()
                            .when(is_home, |main| main.child(self.faulty_terminal.clone()))
                            .child(page)
                            .when_some(editor_closing_progress, |root, progress| {
                                root.child(self.render_editor(progress, cx))
                            })
                            .when_some(confirm_closing_progress, |root, (target, progress)| {
                                root.child(
                                    crate::ui::components::confirm_dialog::render_confirm_dialog(
                                        &target, progress, on_cancel, on_confirm, cx,
                                    ),
                                )
                            }),
                    ),
            )
            .child(crate::ui::components::cursor_tooltip::overlay(cx))
            .map(|this| {
                if cfg!(debug_assertions) {
                    this.child(crate::ui::components::fps::fps_monitor(window, cx))
                } else {
                    this
                }
            })
    }
}

impl AppView {
    fn breadcrumb(&self, cx: &mut Context<Self>) -> AnyElement {
        match &self.route {
            Route::Dns => self.two_level_breadcrumb(
                t!("nav.modules"),
                Route::Modules,
                t!("modules.dns_title"),
                cx,
            ),
            Route::TgProxy => self.two_level_breadcrumb(
                t!("nav.modules"),
                Route::Modules,
                t!("modules.tg_proxy_title"),
                cx,
            ),
            Route::Category(id) => {
                let name = self
                    .state
                    .read(cx)
                    .config
                    .categories
                    .iter()
                    .find(|category| &category.id == id)
                    .map_or_else(
                        || t!("strategies.category_name").to_string(),
                        |c| c.name.clone(),
                    );
                self.two_level_breadcrumb(t!("nav.strategies"), Route::Strategies, name, cx)
            }
            Route::StrategyProbe => self.two_level_breadcrumb(
                t!("nav.strategies"),
                Route::Strategies,
                t!("probe.title"),
                cx,
            ),
            route => {
                let label: SharedString = match route {
                    Route::Home => t!("nav.home").into(),
                    Route::Modules => t!("nav.modules").into(),
                    Route::Strategies => t!("nav.strategies").into(),
                    Route::Filters => t!("nav.filters").into(),
                    Route::Placeholders => t!("nav.placeholders").into(),
                    Route::Logs => t!("nav.logs").into(),
                    Route::Settings => t!("nav.settings").into(),
                    Route::About => t!("nav.about").into(),
                    _ => String::new().into(),
                };
                div()
                    .text_color(foreground())
                    .child(label)
                    .into_any_element()
            }
        }
    }

    fn two_level_breadcrumb(
        &self,
        parent_label: impl Into<SharedString>,
        parent_route: Route,
        current_label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let parent_label: SharedString = parent_label.into();
        let current_label: SharedString = current_label.into();
        let hover_key: SharedString =
            SharedString::from(format!("breadcrumb-parent-hover-{parent_label}"));
        let hover = crate::ui::foundation::hover_motion::progress(&hover_key, cx);
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .id(SharedString::from(format!(
                        "breadcrumb-parent-{parent_label}"
                    )))
                    .cursor_pointer()
                    .text_color(mix_color(foreground(), accent(), hover))
                    .on_hover(move |hovered, window, cx| {
                        crate::ui::foundation::hover_motion::set_hovered(
                            hover_key.clone(),
                            *hovered,
                            window,
                            cx,
                        );
                    })
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(cx.listener({
                        let parent_route = parent_route.clone();
                        move |this, _, _, cx| {
                            this.navigate(parent_route.clone(), cx);
                        }
                    }))
                    .child(parent_label),
            )
            .child(div().text_color(muted_foreground().opacity(0.5)).child("/"))
            .child(div().text_color(foreground()).child(current_label))
            .into_any_element()
    }

    fn sidebar(
        &self,
        progress: f32,
        acrylic_progress: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let main = [
            (
                Route::Home,
                SharedString::from(t!("nav.home")),
                "icons/house.svg",
            ),
            (
                Route::Modules,
                SharedString::from(t!("nav.modules")),
                "icons/boxes.svg",
            ),
            (
                Route::Strategies,
                SharedString::from(t!("nav.strategies")),
                "icons/layers.svg",
            ),
            (
                Route::Filters,
                SharedString::from(t!("nav.filters")),
                "icons/funnel.svg",
            ),
            (
                Route::Placeholders,
                SharedString::from(t!("nav.placeholders")),
                "icons/file-code.svg",
            ),
            (
                Route::Logs,
                SharedString::from(t!("nav.logs")),
                "icons/logs.svg",
            ),
        ];
        let footer = [
            (
                Route::Settings,
                SharedString::from(t!("nav.settings")),
                "icons/settings.svg",
            ),
            (
                Route::About,
                SharedString::from(t!("nav.about")),
                "icons/info.svg",
            ),
        ];
        let bar_alpha = (1.0 - acrylic_progress).clamp(0.0, 1.0);
        div()
            .h_full()
            .w(px(SIDEBAR_COLLAPSED_WIDTH
                + (SIDEBAR_EXPANDED_WIDTH - SIDEBAR_COLLAPSED_WIDTH)
                    * progress))
            .bg(card_color().opacity(bar_alpha))
            .flex_shrink_0()
            .relative()
            .flex()
            .flex_col()
            .justify_between()
            .overflow_hidden()
            .child(
                div()
                    .p(SHELL_SPACING)
                    .flex()
                    .flex_col()
                    .gap(SHELL_SPACING)
                    .children(main.into_iter().map(|(route, label, icon)| {
                        self.nav_item(route, label, icon, progress, window, cx)
                    })),
            )
            .child(
                div()
                    .p(SHELL_SPACING)
                    .border_t_1()
                    .border_color(border().opacity(0.8))
                    .flex()
                    .flex_col()
                    .gap(SHELL_SPACING)
                    .children(footer.into_iter().map(|(route, label, icon)| {
                        self.nav_item(route, label, icon, progress, window, cx)
                    })),
            )
    }

    fn nav_item(
        &self,
        route: Route,
        label: SharedString,
        icon: &'static str,
        progress: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_selected = self.route == route
            || matches!(
                (&self.route, &route),
                (Route::Category(_), Route::Strategies)
            );
        let is_hovered = self.hovered_route.as_ref() == Some(&route) && !is_selected;
        let is_unhovered = self.unhovered_route.as_ref() == Some(&route) && !is_selected;
        let item_label = label.clone();
        let tooltip_source = ElementId::Name(format!("nav-tooltip-{label}").into());
        let hover_tooltip_source = tooltip_source.clone();
        let pressed_tooltip_source = tooltip_source.clone();
        let tooltip_label = label.clone();
        let collapsed = progress < 0.5;

        let selected_alpha = if is_selected {
            let elapsed = self.selected_at.elapsed().as_secs_f32();
            (elapsed / 0.15).clamp(0.0, 1.0)
        } else if self.deselected_route.as_ref() == Some(&route) {
            if let Some(at) = self.deselected_at {
                (1.0 - at.elapsed().as_secs_f32() / 0.15).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let hover_alpha = if is_hovered {
            if let Some(at) = self.hovered_at {
                (at.elapsed().as_secs_f32() / 0.15).clamp(0.0, 1.0)
            } else {
                1.0
            }
        } else if is_unhovered {
            if let Some(at) = self.unhovered_at {
                (1.0 - at.elapsed().as_secs_f32() / 0.15).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let active_t = if selected_alpha > 0.001 {
            selected_alpha
        } else {
            hover_alpha
        };

        let target_foreground = if selected_alpha > 0.001 {
            accent_foreground()
        } else {
            accent()
        };
        let icon_color = mix_color(foreground(), target_foreground, active_t);

        if (selected_alpha > 0.0 && selected_alpha < 1.0)
            || (hover_alpha > 0.0 && hover_alpha < 1.0)
        {
            cx.on_next_frame(window, |_, _, cx| cx.notify());
        }

        div()
            .id(SharedString::from(format!("nav-{label}")))
            .relative()
            .h(SHELL_CONTROL_SIZE)
            .w_full()
            .flex()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .when(selected_alpha > 0.001, |this| {
                this.bg(accent().opacity(selected_alpha))
            })
            .when(selected_alpha <= 0.001 && hover_alpha > 0.001, |this| {
                this.bg(mix_color(card_color().alpha(1.0), accent(), 0.34).opacity(hover_alpha))
            })
            .on_hover(cx.listener({
                let route = route.clone();
                move |this, is_hovered, window, cx| {
                    this.set_hovered_route(route.clone(), *is_hovered, cx);
                    if collapsed {
                        crate::ui::components::cursor_tooltip::set_hovered(
                            hover_tooltip_source.clone(),
                            tooltip_label.clone(),
                            *is_hovered,
                            window,
                            cx,
                        );
                    } else {
                        crate::ui::components::cursor_tooltip::hide_source(
                            &hover_tooltip_source,
                            window,
                            cx,
                        );
                    }
                }
            }))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                crate::ui::components::cursor_tooltip::hide_source(
                    &pressed_tooltip_source,
                    window,
                    cx,
                );
            })
            .on_click(cx.listener({
                let route = route.clone();
                move |this, _, window, cx| {
                    crate::ui::components::cursor_tooltip::hide(window, cx);
                    this.navigate(route.clone(), cx);
                }
            }))
            .child(
                div()
                    .size(SHELL_CONTROL_SIZE)
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(svg().path(icon).size(px(14.4)).text_color(icon_color)),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(icon_color.opacity(progress))
                    .ml(px(2. * progress))
                    .truncate()
                    .child(item_label),
            )
            .into_any_element()
    }
}

#[allow(clippy::too_many_arguments)]
fn titlebar(
    is_maximized: bool,
    progress: f32,
    acrylic_progress: f32,
    breadcrumb: AnyElement,
    minimize_on_close: bool,
    state: Entity<AppState>,
    toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let (app_update, is_updating, update_progress) = {
        let s = state.read(cx);
        (
            s.app_update.clone(),
            s.is_updating,
            s.update_download_progress,
        )
    };
    let update_button =
        titlebar_update_button(app_update, is_updating, update_progress, state.clone(), cx);
    let probe_button = titlebar_probe_button(state.clone(), cx);

    let bar_alpha = (1.0 - acrylic_progress).clamp(0.0, 1.0);
    div()
        .id("titlebar")
        .h(px(SIDEBAR_COLLAPSED_WIDTH))
        .w_full()
        .bg(card_color().opacity(bar_alpha))
        .flex()
        .items_center()
        .justify_between()
        .window_control_area(WindowControlArea::Drag)
        .relative()
        .child(
            div()
                .size(px(SIDEBAR_COLLAPSED_WIDTH))
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .child(
                    titlebar_button("sidebar-toggle", false, cx)
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_click(toggle)
                        .child(sidebar_icon(progress)),
                ),
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .child(breadcrumb),
        )
        .child(
            div()
                .flex_1()
                .h_full()
                .window_control_area(WindowControlArea::Drag),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(SHELL_SPACING)
                .pr(SHELL_SPACING)
                .flex_shrink_0()
                .children(probe_button)
                .children(update_button)
                .child(
                    titlebar_button("minimize", false, cx)
                        .window_control_area(WindowControlArea::Min)
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            window.minimize_window();
                        })
                        .child(titlebar_icon("icons/window-minimize.svg")),
                )
                .child(
                    titlebar_button("maximize", false, cx)
                        .window_control_area(WindowControlArea::Max)
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            if is_maximized {
                                #[cfg(windows)]
                                crate::tray::restore_main_window();
                                #[cfg(not(windows))]
                                window.zoom_window();
                            } else {
                                window.zoom_window();
                            }
                        })
                        .child(titlebar_icon(if is_maximized {
                            "icons/window-restore.svg"
                        } else {
                            "icons/window-maximize.svg"
                        })),
                )
                .child(
                    titlebar_button("close", true, cx)
                        .window_control_area(WindowControlArea::Close)
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_click(move |_, window, cx| {
                            if minimize_on_close {
                                crate::ui::foundation::hover_motion::clear_all_hovers_app(cx);
                                window.refresh();
                                #[cfg(windows)]
                                crate::tray::hide_main_window();
                                #[cfg(not(windows))]
                                window.minimize_window();
                            } else {
                                let deferred =
                                    state.update(cx, |state, cx| state.defer_quit_for_probe(cx));
                                if !deferred {
                                    cx.quit();
                                }
                            }
                        })
                        .child(destructive_titlebar_icon("icons/window-close.svg", cx)),
                ),
        )
        .into_any_element()
}

fn titlebar_probe_button(state: Entity<AppState>, cx: &App) -> Option<AnyElement> {
    let crate::app_state::StrategyProbeState::Running(progress) =
        state.read(cx).strategy_probe.clone()
    else {
        return None;
    };
    let label = if progress.category_name.is_empty() {
        t!("probe.running").to_string()
    } else {
        format!(
            "{} {}/{}",
            progress.category_name,
            progress.candidate_index + 1,
            progress.candidate_total
        )
    };
    Some(
        Button::new("titlebar-probe", label, cx)
            .secondary()
            .small()
            .icon_prefix("icons/square-stop.svg")
            .tooltip(t!("probe.cancel"))
            .on_click(move |_, _, cx| {
                state.update(cx, |state, cx| state.cancel_strategy_probe(cx));
            })
            .into_any_element(),
    )
}

fn titlebar_update_button(
    app_update: Option<crate::services::updater::AppUpdateInfo>,
    is_updating: bool,
    download_progress: Option<f32>,
    state: Entity<AppState>,
    cx: &App,
) -> Option<AnyElement> {
    if app_update.is_none() && !is_updating {
        return None;
    }

    let icon = if let Some(progress) = download_progress.filter(|_| is_updating) {
        crate::ui::components::badge::progress_ring(progress)
    } else if is_updating {
        svg()
            .path("icons/refresh-cw.svg")
            .size_4()
            .text_color(accent())
            .with_animation(
                "titlebar-update-downloading",
                Animation::new(Duration::from_millis(850)).repeat(),
                |icon, delta| {
                    icon.with_transformation(Transformation::rotate(
                        crate::ui::foundation::motion::refresh_rotation(delta),
                    ))
                },
            )
            .into_any_element()
    } else {
        svg()
            .path("icons/cloud-download.svg")
            .size_4()
            .text_color(green())
            .with_animation(
                "titlebar-update-available-pulse",
                Animation::new(crate::ui::foundation::motion::UPDATE_PULSE_MOTION).repeat(),
                |icon, delta| {
                    icon.opacity(crate::ui::foundation::motion::update_pulse_opacity(delta))
                },
            )
            .into_any_element()
    };

    let tooltip_text = if is_updating {
        t!("titlebar.updating")
    } else if let Some(ref update) = app_update {
        t!("titlebar.update_to", version = update.new_version.as_str())
    } else {
        t!("titlebar.update_app")
    };

    let (button, hover_key) = titlebar_button_base("titlebar-update-btn", false, cx);
    let button = button
        .when(is_updating, |button| button.cursor_default())
        .when(!is_updating, |button| {
            button
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |_, _, cx| {
                    state.update(cx, |s, cx| s.trigger_app_update(cx));
                })
        })
        .child(icon);

    Some(
        crate::ui::components::cursor_tooltip::attach_with_hover_motion(
            button,
            ElementId::Name("titlebar-update-tooltip".into()),
            hover_key,
            tooltip_text,
        )
        .into_any_element(),
    )
}

fn titlebar_button(id: &'static str, is_destructive: bool, cx: &App) -> Stateful<Div> {
    let (button, hover_key) = titlebar_button_base(id, is_destructive, cx);
    button.on_hover(move |hovered, window, cx| {
        crate::ui::foundation::hover_motion::set_hovered(hover_key.clone(), *hovered, window, cx);
    })
}

fn titlebar_button_base(
    id: &'static str,
    is_destructive: bool,
    cx: &App,
) -> (Stateful<Div>, SharedString) {
    let hover_key = SharedString::from(format!("titlebar-button-{id}"));
    let hover = crate::ui::foundation::hover_motion::progress(&hover_key, cx);
    let button = div()
        .id(id)
        .when(is_destructive, |btn| btn.group("titlebar-destructive"))
        .size(SHELL_CONTROL_SIZE)
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .cursor_pointer()
        .bg(if is_destructive {
            destructive().opacity(0.15 * hover)
        } else {
            secondary().opacity(0.70 * hover)
        })
        .text_color(mix_color(
            foreground(),
            if is_destructive {
                destructive()
            } else {
                foreground()
            },
            hover,
        ))
        .active(move |style| {
            if is_destructive {
                style.bg(destructive()).text_color(paper())
            } else {
                style.bg(secondary().opacity(0.95)).text_color(foreground())
            }
        });
    (button, hover_key)
}

fn titlebar_icon(path: &'static str) -> Svg {
    svg().path(path).size(px(14.)).text_color(foreground())
}

fn destructive_titlebar_icon(path: &'static str, cx: &App) -> Div {
    let hover = crate::ui::foundation::hover_motion::progress(
        &SharedString::from("titlebar-button-close"),
        cx,
    );
    div()
        .relative()
        .size_4()
        .child(
            div()
                .id("titlebar-close-normal-icon")
                .absolute()
                .inset_0()
                .child(
                    svg()
                        .path(path)
                        .size_4()
                        .text_color(foreground())
                        .opacity(1.0 - hover),
                ),
        )
        .child(
            div()
                .id("titlebar-close-hover-icon")
                .absolute()
                .inset_0()
                .group_active("titlebar-destructive", |style| style.invisible())
                .child(
                    svg()
                        .path(path)
                        .size_4()
                        .text_color(destructive())
                        .opacity(hover),
                ),
        )
        .child(
            div()
                .id("titlebar-close-pressed-icon")
                .absolute()
                .inset_0()
                .invisible()
                .group_active("titlebar-destructive", |style| style.visible())
                .child(svg().path(path).size_4().text_color(paper())),
        )
}
fn sidebar_icon(progress: f32) -> Div {
    div()
        .relative()
        .size_4()
        .child(
            svg()
                .path("icons/panel-left-close.svg")
                .size(px(14.))
                .text_color(foreground())
                .opacity(progress),
        )
        .child(
            svg()
                .absolute()
                .inset_0()
                .path("icons/panel-left-open.svg")
                .size(px(14.))
                .text_color(foreground())
                .opacity(1. - progress),
        )
}

fn page_transition(content: AnyElement, revision: u64) -> AnyElement {
    let page = div().size_full().relative().child(content);
    if revision == 0 {
        return page.into_any_element();
    }
    page.with_animation(
        ElementId::NamedInteger("page-enter-transition".into(), revision),
        Animation::new(Duration::from_millis(320))
            .with_easing(|progress| 1.0 - (1.0 - progress).powi(5)),
        |page, progress| page.opacity(progress).top(px(8.0 * (1.0 - progress))),
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{acrylic_motion, sidebar_motion};

    #[test]
    fn sidebar_starts_expanded() {
        assert_eq!(sidebar_motion(false).sample(), (1., false));
        assert_eq!(sidebar_motion(true).sample(), (0., false));
    }

    #[test]
    fn acrylic_motion_samples_correctly() {
        assert_eq!(acrylic_motion(true).sample(), (1., false));
        assert_eq!(acrylic_motion(false).sample(), (0., false));
    }
}
