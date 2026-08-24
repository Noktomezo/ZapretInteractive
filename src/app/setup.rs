use super::*;

impl AppView {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        let config = state.read(cx).config.clone();
        let input = |value: String, window: &mut Window, cx: &mut App| {
            cx.new(|cx| {
                let mut state = TextInputState::new(window, cx);
                state.set_value(value, cx);
                state
            })
        };
        let faulty_terminal = cx.new(|cx| FaultyTerminal::new(window, cx));
        let primary_input = input(String::new(), window, cx);
        let secondary_input = input(String::new(), window, cx);
        let text_area =
            cx.new(|cx| crate::ui::components::text_area::TextAreaState::new(window, cx));
        let tcp_input = input(config.global_ports.tcp, window, cx);
        let udp_input = input(config.global_ports.udp, window, cx);
        let tg_port_input = input(config.tg_ws_proxy_port.to_string(), window, cx);
        let tg_secret = if config.tg_ws_proxy_secret.trim().is_empty() {
            let generated = uuid::Uuid::new_v4().simple().to_string();
            state.update(cx, |s, cx| {
                s.set_tg_settings(config.tg_ws_proxy_port.to_string(), generated.clone(), cx);
            });
            generated
        } else {
            config.tg_ws_proxy_secret
        };
        let tg_secret_input = input(tg_secret, window, cx);
        let discord_choices = vec![
            DropdownChoice::new("none", "Нет").with_icon("icons/circle-off.svg"),
            DropdownChoice::new("playing", "Играет").with_icon("icons/gamepad-2.svg"),
            DropdownChoice::new("listening", "Слушает").with_icon("icons/headphones.svg"),
            DropdownChoice::new("watching", "Смотрит").with_icon("icons/clapperboard.svg"),
            DropdownChoice::new("competing", "Соревнуется").with_icon("icons/trophy.svg"),
        ];
        let discord_selected = if config.discord_presence_enabled {
            match config.discord_presence_activity_type {
                crate::domain::DiscordActivity::Playing => 1,
                crate::domain::DiscordActivity::Listening => 2,
                crate::domain::DiscordActivity::Watching => 3,
                crate::domain::DiscordActivity::Competing => 4,
            }
        } else {
            0
        };
        let dropdown_motion = cx.new(|_| DropdownMotion::default());
        let discord_dropdown =
            cx.new(|_| DropdownState::new(discord_choices, discord_selected, dropdown_motion));
        let dns_choices = vec![
            DropdownChoice::new("77.88.8.8", "Yandex · 77.88.8.8"),
            DropdownChoice::new("1.1.1.1", "Cloudflare · 1.1.1.1"),
            DropdownChoice::new("8.8.8.8", "Google · 8.8.8.8"),
        ];
        let bootstrap = config
            .dns_bootstrap_resolvers
            .first()
            .map(String::as_str)
            .unwrap_or("77.88.8.8");
        let dns_selected = dns_choices
            .iter()
            .position(|choice| choice.value.as_ref() == bootstrap)
            .unwrap_or(0);
        let dns_motion = cx.new(|_| DropdownMotion::default());
        let dns_dropdown = cx.new(|_| DropdownState::new(dns_choices, dns_selected, dns_motion));
        let theme_choices = vec![
            DropdownChoice::new("system", t!("settings.theme_system"))
                .with_icon("icons/laptop.svg"),
            DropdownChoice::new("dark", t!("settings.theme_dark")).with_icon("icons/moon-star.svg"),
            DropdownChoice::new("light", t!("settings.theme_light")).with_icon("icons/sun.svg"),
        ];
        let theme_selected = match config.theme {
            crate::domain::ThemePreference::System => 0,
            crate::domain::ThemePreference::Dark => 1,
            crate::domain::ThemePreference::Light => 2,
        };
        let theme_motion = cx.new(|_| DropdownMotion::default());
        let theme_dropdown =
            cx.new(|_| DropdownState::new(theme_choices, theme_selected, theme_motion));

        let language_choices = vec![
            DropdownChoice::new("system", t!("settings.lang_system")).with_icon("icons/laptop.svg"),
            DropdownChoice::new("ru", "Русский").with_icon("icons/flags/ru.png"),
            DropdownChoice::new("en", "English").with_icon("icons/flags/gb.png"),
        ];
        let language_selected = match config.language {
            crate::domain::LanguagePreference::System => 0,
            crate::domain::LanguagePreference::Ru => 1,
            crate::domain::LanguagePreference::En => 2,
        };
        let language_motion = cx.new(|_| DropdownMotion::default());
        let language_dropdown =
            cx.new(|_| DropdownState::new(language_choices, language_selected, language_motion));
        let tcp_subscription = tcp_input.clone();
        let udp_subscription = udp_input.clone();
        let tg_port_subscription = tg_port_input.clone();
        let tg_secret_subscription = tg_secret_input.clone();
        cx.new(|cx| {
            let mut this = Self {
                state,
                route: Route::Home,
                editor: None,
                primary_input,
                secondary_input,
                text_area,
                tcp_input,
                udp_input,
                tg_port_input,
                tg_secret_input,
                theme_dropdown: theme_dropdown.clone(),
                language_dropdown: language_dropdown.clone(),
                discord_dropdown: discord_dropdown.clone(),
                dns_dropdown: dns_dropdown.clone(),
                faulty_terminal,
                sidebar_collapsed: config.sidebar_collapsed,
                tg_info_expanded: false,
                probe_expanded_category: None,
                sidebar_motion: sidebar_motion(config.sidebar_collapsed),
                acrylic_motion: acrylic_motion(config.acrylic_material),
                page_revision: 0,
                _subscriptions: Vec::new(),
                selected_at: Instant::now() - Duration::from_secs(10),
                deselected_route: None,
                deselected_at: None,
                hovered_route: None,
                unhovered_route: None,
                hovered_at: None,
                unhovered_at: None,
                categories_scroll_handle: UniformListScrollHandle::new(),
                filters_scroll_handle: UniformListScrollHandle::new(),
                placeholders_scroll_handle: UniformListScrollHandle::new(),
                category_strategies_list_state: ListState::new(0, ListAlignment::Top, px(240.0))
                    .measure_all(),
                probe_results_list_states: HashMap::new(),
                current_viewed_category: None,
                closing_editor: None,
                closing_confirm: None,
            };
            let theme_dd = theme_dropdown.clone();
            this._subscriptions.push(cx.subscribe(
                &theme_dropdown,
                move |this: &mut AppView, _, _: &DropdownEvent, cx| {
                    let selection = theme_dd.read(cx).selected_value().map(ToOwned::to_owned);
                    let state = this.state.clone();
                    state.update(cx, |state, cx| match selection.as_deref() {
                        Some("dark") => state.set_theme(crate::domain::ThemePreference::Dark, cx),
                        Some("light") => state.set_theme(crate::domain::ThemePreference::Light, cx),
                        _ => state.set_theme(crate::domain::ThemePreference::System, cx),
                    });
                },
            ));
            let language_dd = language_dropdown.clone();
            this._subscriptions.push(cx.subscribe(
                &language_dropdown,
                move |this: &mut AppView, _, _: &DropdownEvent, cx| {
                    let selection = language_dd.read(cx).selected_value().map(ToOwned::to_owned);
                    let state = this.state.clone();
                    state.update(cx, |state, cx| match selection.as_deref() {
                        Some("ru") => state.set_language(crate::domain::LanguagePreference::Ru, cx),
                        Some("en") => state.set_language(crate::domain::LanguagePreference::En, cx),
                        _ => state.set_language(crate::domain::LanguagePreference::System, cx),
                    });
                },
            ));
            let dropdown = discord_dropdown.clone();
            this._subscriptions.push(cx.subscribe(
                &discord_dropdown,
                move |this: &mut AppView, _, _: &DropdownEvent, cx| {
                    let selection = dropdown.read(cx).selected_value().map(ToOwned::to_owned);
                    let state = this.state.clone();
                    state.update(cx, |state, cx| match selection.as_deref() {
                        Some("playing") => {
                            state.set_discord_presence(true, cx);
                            state.set_discord_activity(crate::domain::DiscordActivity::Playing, cx);
                        }
                        Some("listening") => {
                            state.set_discord_presence(true, cx);
                            state.set_discord_activity(
                                crate::domain::DiscordActivity::Listening,
                                cx,
                            );
                        }
                        Some("watching") => {
                            state.set_discord_presence(true, cx);
                            state
                                .set_discord_activity(crate::domain::DiscordActivity::Watching, cx);
                        }
                        Some("competing") => {
                            state.set_discord_presence(true, cx);
                            state.set_discord_activity(
                                crate::domain::DiscordActivity::Competing,
                                cx,
                            );
                        }
                        _ => state.set_discord_presence(false, cx),
                    });
                },
            ));
            let dns_selection = dns_dropdown.clone();
            this._subscriptions.push(cx.subscribe(
                &dns_dropdown,
                move |this: &mut AppView, _, _: &DropdownEvent, cx| {
                    let Some(resolver) = dns_selection
                        .read(cx)
                        .selected_value()
                        .map(ToOwned::to_owned)
                    else {
                        return;
                    };
                    this.state.update(cx, |state, cx| {
                        state.set_dns_bootstrap_resolvers(&[resolver], cx);
                    });
                },
            ));
            let observed_state = this.state.clone();
            this._subscriptions.push(cx.observe(
                &observed_state,
                |this: &mut AppView, state, cx| {
                    let (acrylic_enabled, status) = {
                        let state = state.read(cx);
                        (state.config.acrylic_material, state.status)
                    };
                    this.acrylic_motion.set_target(f32::from(acrylic_enabled));
                    this.faulty_terminal
                        .update(cx, |terminal, _| terminal.set_status(status));
                    cx.notify();
                },
            ));
            this._subscriptions.push(cx.subscribe(
                &tcp_subscription,
                move |this: &mut AppView, _, event: &TextInputEvent, cx| {
                    if matches!(event, TextInputEvent::Blur | TextInputEvent::PressEnter) {
                        let tcp = this.tcp_input.read(cx).value().to_owned();
                        let udp = this.udp_input.read(cx).value().to_owned();
                        this.state
                            .update(cx, |state, cx| state.set_ports(tcp, udp, cx));
                    }
                },
            ));
            this._subscriptions.push(cx.subscribe(
                &udp_subscription,
                move |this: &mut AppView, _, event: &TextInputEvent, cx| {
                    if matches!(event, TextInputEvent::Blur | TextInputEvent::PressEnter) {
                        let tcp = this.tcp_input.read(cx).value().to_owned();
                        let udp = this.udp_input.read(cx).value().to_owned();
                        this.state
                            .update(cx, |state, cx| state.set_ports(tcp, udp, cx));
                    }
                },
            ));
            this._subscriptions.push(cx.subscribe(
                &tg_port_subscription,
                move |this: &mut AppView, _, event: &TextInputEvent, cx| {
                    if matches!(event, TextInputEvent::Blur | TextInputEvent::PressEnter) {
                        this.commit_tg_settings(cx);
                    }
                },
            ));
            this._subscriptions.push(cx.subscribe(
                &tg_secret_subscription,
                move |this: &mut AppView, _, event: &TextInputEvent, cx| {
                    if matches!(event, TextInputEvent::Blur | TextInputEvent::PressEnter) {
                        this.commit_tg_settings(cx);
                    }
                },
            ));
            this
        })
    }

    pub fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        if self.route == route {
            return;
        }
        crate::ui::foundation::hover_motion::clear_all_hovers_app(cx);
        let now = Instant::now();
        self.deselected_route = Some(self.route.clone());
        self.deselected_at = Some(now);
        self.route = route;
        self.selected_at = now;
        self.page_revision = self.page_revision.wrapping_add(1);
        self.faulty_terminal.update(cx, |terminal, _| {
            terminal.set_active(matches!(self.route, Route::Home))
        });
        cx.notify();
    }

    pub(crate) fn set_hovered_route(
        &mut self,
        route: Route,
        is_hovered: bool,
        cx: &mut Context<Self>,
    ) {
        let now = Instant::now();
        if is_hovered {
            if self.hovered_route.as_ref() != Some(&route) {
                if let Some(old) = self.hovered_route.take() {
                    self.unhovered_route = Some(old);
                    self.unhovered_at = Some(now);
                }
                self.hovered_route = Some(route);
                self.hovered_at = Some(now);
                cx.notify();
            }
        } else if self.hovered_route.as_ref() == Some(&route) {
            self.unhovered_route = Some(route);
            self.unhovered_at = Some(now);
            self.hovered_route = None;
            self.hovered_at = None;
            cx.notify();
        }
    }

    fn commit_tg_settings(&self, cx: &mut Context<Self>) {
        let port = self.tg_port_input.read(cx).value().to_owned();
        let secret = self.tg_secret_input.read(cx).value().to_owned();
        self.state
            .update(cx, |state, cx| state.set_tg_settings(port, secret, cx));
    }
}
