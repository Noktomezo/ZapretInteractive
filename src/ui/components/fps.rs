use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::*;

use crate::ui::components::backdrop_blur::backdrop_blur;
use crate::ui::components::cursor_tooltip;
use crate::ui::components::dashed_outline::dashed_outline;
use crate::ui::foundation::colors::{
    accent, card, destructive, foreground, muted_foreground, success, warning,
};
use crate::ui::foundation::hover_motion;
use crate::ui::foundation::i18n::t;
use crate::ui::foundation::motion::{ScalarTransition, mix_color};

const DEFAULT_FRAME_BUDGET: Duration = Duration::from_nanos(16_666_667); // 60Hz
const DEFAULT_CAPACITY: usize = 120;
const DEFAULT_RESOURCE_INTERVAL: Duration = Duration::from_millis(500);
const AXIS_DECAY: f32 = 0.04;
const HUD_WIDTH: Pixels = px(208.0);
const HUD_HEIGHT: Pixels = px(154.0);
const HUD_COLLAPSED_WIDTH: Pixels = px(124.0);
const HUD_COLLAPSED_HEIGHT: Pixels = px(28.0);
const HUD_MARGIN: Pixels = px(16.0);
const HUD_TOP: Pixels = px(56.0);
const TEXT_SIZE: Pixels = px(11.0);
const TRACE_OPACITY: f32 = 1.0;
const CHART_HEIGHT: Pixels = px(42.0);
const CHART_LINE_WIDTH: Pixels = px(1.6);
const CHART_SAMPLE_INTERVAL: Duration = Duration::from_millis(50);
const READOUT_INTERVAL: Duration = Duration::from_millis(500);
const FPS_TOLERANCE: f32 = 0.95;
const FPS_WINDOW: Duration = Duration::from_secs(1);
const DEFAULT_FONT: &str = "IBM Plex Mono";
const MORPH_DURATION: Duration = Duration::from_millis(220);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FpsStyle {
    pub background: Hsla,
    pub foreground: Hsla,
    pub muted: Hsla,
    pub good: Hsla,
    pub warn: Hsla,
    pub bad: Hsla,
}

impl Default for FpsStyle {
    fn default() -> Self {
        Self {
            background: card().into(),
            foreground: foreground().into(),
            muted: muted_foreground().into(),
            good: success().into(),
            warn: warning().into(),
            bad: destructive().into(),
        }
    }
}

impl FpsStyle {
    pub fn level_color(&self, frame_secs: f32, budget_secs: f32) -> Hsla {
        if frame_secs <= budget_secs {
            self.good
        } else if frame_secs <= budget_secs * 2.0 {
            self.warn
        } else {
            self.bad
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameSample {
    pub draw: Duration,
}

pub(crate) struct FrameSampler {
    samples: VecDeque<FrameSample>,
    frame_times: VecDeque<Instant>,
    capacity: usize,
    last_tick: Option<Instant>,
    last_sample_tick: Option<Instant>,
    accumulated_draw: Duration,
    accumulated_count: u32,
}

impl FrameSampler {
    pub(crate) fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            samples: VecDeque::with_capacity(capacity),
            frame_times: VecDeque::new(),
            capacity,
            last_tick: None,
            last_sample_tick: None,
            accumulated_draw: Duration::ZERO,
            accumulated_count: 0,
        }
    }

    pub(crate) fn tick(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_tick {
            let draw = now.duration_since(last);
            self.frame_times.push_back(now);
            self.accumulated_draw += draw;
            self.accumulated_count += 1;

            let sample_due = self
                .last_sample_tick
                .is_none_or(|at| now.duration_since(at) >= CHART_SAMPLE_INTERVAL);
            if sample_due {
                let sample_draw = if self.accumulated_count > 0 {
                    self.accumulated_draw / self.accumulated_count
                } else {
                    draw
                };
                if self.samples.len() == self.capacity {
                    self.samples.pop_front();
                }
                self.samples.push_back(FrameSample { draw: sample_draw });
                self.last_sample_tick = Some(now);
                self.accumulated_draw = Duration::ZERO;
                self.accumulated_count = 0;
            }
        }
        self.last_tick = Some(now);

        while let Some(oldest) = self.frame_times.front() {
            if now.duration_since(*oldest) > FPS_WINDOW {
                self.frame_times.pop_front();
            } else {
                break;
            }
        }
    }

    pub(crate) fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        while self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
    }

    pub(crate) fn fps(&self) -> f32 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        let (Some(oldest), Some(newest)) = (self.frame_times.front(), self.frame_times.back())
        else {
            return 0.0;
        };
        let span = newest.duration_since(*oldest).as_secs_f32();
        if span <= 0.0 {
            return 0.0;
        }
        (self.frame_times.len() - 1) as f32 / span
    }

    pub(crate) fn samples(&self) -> impl ExactSizeIterator<Item = &FrameSample> {
        self.samples.iter()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    pub(crate) fn over_budget_ratio(&self, budget: Duration) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let over = self
            .samples
            .iter()
            .filter(|sample| sample.draw > budget)
            .count();
        over as f32 / self.samples.len() as f32
    }

    pub(crate) fn mean_draw(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.samples.iter().map(|sample| sample.draw).sum();
        total / self.samples.len() as u32
    }

    pub(crate) fn peak_draw(&self) -> Duration {
        self.samples
            .iter()
            .map(|sample| sample.draw)
            .max()
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ResourceSample {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
}

#[cfg(windows)]
pub(crate) struct ResourceProbe {
    last_sample_time: Instant,
    last_user_time: u64,
    last_kernel_time: u64,
    num_cpus: f32,
}

#[cfg(windows)]
impl ResourceProbe {
    pub(crate) fn new() -> Option<Self> {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

        let num_cpus = std::thread::available_parallelism()
            .map(|p| p.get() as f32)
            .unwrap_or(1.0);

        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();

        // SAFETY: Calling GetProcessTimes with valid pointers for the current pseudo-handle.
        let success = unsafe {
            GetProcessTimes(
                GetCurrentProcess(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        };

        if success.is_ok() {
            let k = ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
            let u = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);
            Some(Self {
                last_sample_time: Instant::now(),
                last_user_time: u,
                last_kernel_time: k,
                num_cpus,
            })
        } else {
            None
        }
    }

    pub(crate) fn sample(&mut self) -> Option<ResourceSample> {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sample_time).as_secs_f64();
        if elapsed < 0.1 {
            return None;
        }

        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();

        // SAFETY: Calling GetProcessTimes with valid pointers.
        let ok = unsafe {
            GetProcessTimes(
                GetCurrentProcess(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        };

        if ok.is_err() {
            return None;
        }

        let k = ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
        let u = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);

        let k_delta = k.saturating_sub(self.last_kernel_time);
        let u_delta = u.saturating_sub(self.last_user_time);

        self.last_sample_time = now;
        self.last_kernel_time = k;
        self.last_user_time = u;

        // Windows FILETIME increments in 100-nanosecond intervals (10,000,000 per second).
        let total_time_sec = (k_delta + u_delta) as f64 / 10_000_000.0;
        let cpu_percent =
            ((total_time_sec / elapsed) / self.num_cpus as f64 * 100.0).clamp(0.0, 100.0) as f32;

        let mut pmc = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };

        // SAFETY: Calling GetProcessMemoryInfo with valid size and pointer.
        let mem_ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) };

        let memory_bytes = if mem_ok.is_ok() {
            pmc.WorkingSetSize as u64
        } else {
            0
        };

        Some(ResourceSample {
            cpu_percent,
            memory_bytes,
        })
    }
}

#[cfg(not(windows))]
pub(crate) struct ResourceProbe;

#[cfg(not(windows))]
impl ResourceProbe {
    pub(crate) fn new() -> Option<Self> {
        None
    }
    pub(crate) fn sample(&mut self) -> Option<ResourceSample> {
        None
    }
}

#[derive(Clone, Copy, Default)]
struct Readout {
    fps: f32,
    frame_millis: f32,
    dropped_percent: f32,
}

pub struct FpsMonitor {
    sampler: FrameSampler,
    readout: Readout,
    readout_at: Option<Instant>,
    frame_budget: Duration,
    continuous: bool,
    show_resources: bool,
    resource_interval: Duration,
    resources: Option<ResourceSample>,
    axis_max: f32,
    resource_task: Option<Task<()>>,
    position: Point<Pixels>,
    drag: Option<HudDrag>,
    collapsed: bool,
    collapse_motion: ScalarTransition,
}

#[derive(Clone, Copy)]
struct HudDrag {
    pointer: Point<Pixels>,
    origin: Point<Pixels>,
}

impl FpsMonitor {
    pub fn new(window: &Window, _cx: &mut Context<Self>) -> Self {
        let frame_budget = DEFAULT_FRAME_BUDGET;
        Self {
            sampler: FrameSampler::new(DEFAULT_CAPACITY),
            readout: Readout::default(),
            readout_at: None,
            frame_budget,
            continuous: true,
            show_resources: true,
            resource_interval: DEFAULT_RESOURCE_INTERVAL,
            resources: None,
            axis_max: frame_budget.as_secs_f32() * 2.0,
            resource_task: None,
            position: default_hud_position(window.viewport_size()),
            drag: None,
            collapsed: false,
            collapse_motion: ScalarTransition::new(0.0, MORPH_DURATION),
        }
    }

    pub fn capacity(mut self, capacity: usize) -> Self {
        self.sampler.set_capacity(capacity);
        self
    }

    pub fn frame_budget(mut self, budget: Duration) -> Self {
        self.frame_budget = budget;
        self.axis_max = budget.as_secs_f32() * 2.0;
        self
    }

    pub fn continuous(mut self, continuous: bool) -> Self {
        self.continuous = continuous;
        self
    }

    pub fn show_resources(mut self, show_resources: bool) -> Self {
        self.show_resources = show_resources;
        self
    }

    pub fn resource_interval(mut self, interval: Duration) -> Self {
        self.resource_interval = interval;
        self
    }

    fn start_resource_sampling(&mut self, cx: &mut Context<Self>) {
        if !self.show_resources || self.resource_task.is_some() {
            return;
        }

        let interval = self.resource_interval;
        self.resource_task = Some(cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            let Some(mut probe) = executor.spawn(async { ResourceProbe::new() }).await else {
                return;
            };

            loop {
                executor.timer(interval).await;
                let (returned, sample) = executor
                    .spawn(async move {
                        let sample = probe.sample();
                        (probe, sample)
                    })
                    .await;
                probe = returned;

                let Some(sample) = sample else { continue };
                let updated = this.update(cx, |this, cx| {
                    this.resources = Some(sample);
                    cx.notify();
                });
                if updated.is_err() {
                    break;
                }
            }
        }));
    }

    fn update_readout(&mut self) {
        let now = Instant::now();
        let due = self
            .readout_at
            .is_none_or(|at| now.duration_since(at) >= READOUT_INTERVAL);
        if !due {
            return;
        }

        self.readout = Readout {
            fps: self.sampler.fps(),
            frame_millis: self.sampler.mean_draw().as_secs_f32() * 1000.0,
            dropped_percent: self.sampler.over_budget_ratio(self.frame_budget) * 100.0,
        };
        self.readout_at = Some(now);
    }

    fn update_axis(&mut self) {
        let floor = self.frame_budget.as_secs_f32() * 2.0;
        let target = self.sampler.peak_draw().as_secs_f32().max(floor);
        self.axis_max = if target > self.axis_max {
            target
        } else {
            self.axis_max + (target - self.axis_max) * AXIS_DECAY
        };
    }

    fn render_chart(&self, style: FpsStyle) -> impl IntoElement {
        let budget = self.frame_budget.as_secs_f32();
        let axis_max = self.axis_max.max(f32::EPSILON);
        let capacity = self.sampler.capacity();
        let samples: Vec<(f32, Hsla)> = self
            .sampler
            .samples()
            .map(|sample| {
                let seconds = sample.draw.as_secs_f32();
                (
                    (seconds / axis_max).clamp(0.0, 1.0),
                    style.level_color(seconds, budget).opacity(TRACE_OPACITY),
                )
            })
            .collect();

        canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window, _| {
                if samples.is_empty() {
                    return;
                }
                let slot = bounds.size.width / capacity as f32;
                let leading = capacity.saturating_sub(samples.len());
                let sample_point = |index: usize, ratio: f32| {
                    point(
                        bounds.origin.x + slot * (leading + index) as f32 + slot / 2.0,
                        bounds.origin.y + bounds.size.height * (1.0 - ratio),
                    )
                };

                let mut start = 0;
                while start + 1 < samples.len() {
                    let color = samples[start + 1].1;
                    let mut path = PathBuilder::stroke(CHART_LINE_WIDTH);
                    path.move_to(sample_point(start, samples[start].0));

                    let mut end = start + 1;
                    while end < samples.len() && samples[end].1 == color {
                        path.line_to(sample_point(end, samples[end].0));
                        end += 1;
                    }

                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                    start = end - 1;
                }
            },
        )
        .absolute()
        .inset_0()
    }

    fn render_chart_panel(&self, style: FpsStyle) -> Div {
        div()
            .relative()
            .overflow_hidden()
            .w_full()
            .h(CHART_HEIGHT)
            .rounded(px(4.0))
            .child(self.render_chart(style))
    }
}

impl Render for FpsMonitor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sampler.tick();
        self.update_readout();
        self.update_axis();
        self.start_resource_sampling(cx);

        let (collapse_t, is_animating) = self.collapse_motion.sample();
        if self.continuous || is_animating {
            window.request_animation_frame();
        }

        let style = FpsStyle::default();
        let budget = self.frame_budget;
        let Readout {
            fps,
            frame_millis,
            dropped_percent: dropped,
        } = self.readout;
        let fps_color = fps_color(fps, budget, style);
        let resources = self.resources.filter(|_| self.show_resources);
        let position = self.position;
        let dragging = self.drag.is_some();
        let monitor = cx.entity().clone();
        let capture_monitor = monitor.clone();

        let current_size = hud_morph_size(collapse_t);

        let capture = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                window.on_mouse_event({
                    let monitor = capture_monitor.clone();
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase != DispatchPhase::Capture || !event.dragging() {
                            return;
                        }
                        let drag_state = monitor.read_with(cx, |m, _| {
                            let (collapse_t, _) = m.collapse_motion.sample();
                            (m.drag, hud_morph_size(collapse_t))
                        });
                        let (Some(drag), hud_size) = drag_state else {
                            return;
                        };
                        let next = point(
                            drag.origin.x + event.position.x - drag.pointer.x,
                            drag.origin.y + event.position.y - drag.pointer.y,
                        );
                        let position = clamp_hud_position(next, window.viewport_size(), hud_size);
                        monitor.update(cx, |monitor, cx| {
                            monitor.position = position;
                            cx.notify();
                        });
                        window.refresh();
                    }
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                    if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                        return;
                    }
                    let dragging =
                        capture_monitor.read_with(cx, |monitor, _| monitor.drag.is_some());
                    if dragging {
                        capture_monitor.update(cx, |monitor, cx| {
                            monitor.drag = None;
                            cx.notify();
                        });
                        window.refresh();
                    }
                });
            },
        )
        .absolute()
        .inset_0();

        let toggle_hk: SharedString = "fps-toggle-collapse-hk".into();
        let toggle_progress = hover_motion::progress(&toggle_hk, cx);
        let toggle_bg = mix_color(
            rgba(0x00000000),
            muted_foreground().opacity(0.15),
            toggle_progress,
        );
        let toggle_fg = mix_color(style.muted.into(), foreground(), toggle_progress);

        let toggle_hk_click = toggle_hk.clone();
        let toggle_button = cursor_tooltip::attach_with_hover_motion(
            div()
                .id("fps-toggle-collapse")
                .size(px(20.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .bg(toggle_bg)
                .active(|btn| btn.bg(accent().opacity(0.25)))
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    hover_motion::clear_hover(&toggle_hk_click, window, cx);
                    this.collapsed = !this.collapsed;
                    this.collapse_motion
                        .set_target(if this.collapsed { 1.0 } else { 0.0 });
                    let target_size = if this.collapsed {
                        size(HUD_COLLAPSED_WIDTH, HUD_COLLAPSED_HEIGHT)
                    } else {
                        size(HUD_WIDTH, HUD_HEIGHT)
                    };
                    this.position =
                        clamp_hud_position(this.position, window.viewport_size(), target_size);
                    cx.notify();
                    window.refresh();
                }))
                .child(
                    svg()
                        .path(if self.collapsed {
                            "icons/chevron-down.svg"
                        } else {
                            "icons/chevron-up.svg"
                        })
                        .size(px(13.0))
                        .text_color(toggle_fg),
                ),
            ElementId::from("fps-toggle-collapse-tooltip"),
            toggle_hk,
            if self.collapsed {
                t("fps.expand")
            } else {
                t("fps.collapse")
            },
        );

        let drag_hk: SharedString = "fps-drag-handle-hk".into();
        let drag_progress = hover_motion::progress(&drag_hk, cx);
        let drag_bg = if dragging {
            accent().opacity(0.18)
        } else {
            mix_color(
                rgba(0x00000000),
                muted_foreground().opacity(0.15),
                drag_progress,
            )
        };
        let drag_fg = if dragging {
            accent()
        } else {
            mix_color(style.muted.into(), foreground(), drag_progress)
        };

        let drag_button = cursor_tooltip::attach_with_hover_motion(
            div()
                .id("fps-drag-handle")
                .size(px(20.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_grab()
                .bg(drag_bg)
                .when(dragging, |btn| btn.cursor_grabbing())
                .active(|btn| btn.bg(accent().opacity(0.25)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, window, cx| {
                        this.drag = Some(HudDrag {
                            pointer: event.position,
                            origin: this.position,
                        });
                        cx.notify();
                        window.refresh();
                    }),
                )
                .child(
                    svg()
                        .path("icons/grip-horizontal.svg")
                        .size(px(14.0))
                        .text_color(drag_fg),
                ),
            ElementId::from("fps-drag-handle-tooltip"),
            drag_hk,
            t("fps.drag_handle"),
        );

        let expanded_alpha = (1.0 - collapse_t * 1.5).clamp(0.0, 1.0);
        let is_collapsed = collapse_t > 0.5;

        let center_or_title = if is_collapsed {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(3.0))
                .font_weight(FontWeight::BOLD)
                .text_size(px(11.0))
                .child(div().text_color(fps_color).child(format!("{fps:.0}")))
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(style.muted)
                        .child(t("fps.fps")),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(style.foreground)
                        .text_size(px(10.5))
                        .opacity(expanded_alpha)
                        .child(t("common.performance")),
                )
                .into_any_element()
        };

        let top_bar = div()
            .flex()
            .w_full()
            .h(px(20.0))
            .items_center()
            .justify_between()
            .gap_1()
            .child(toggle_button)
            .child(center_or_title)
            .child(drag_button);

        let details_opacity = (1.0 - collapse_t * 2.0).clamp(0.0, 1.0);
        let show_details = collapse_t < 0.999;

        div()
            .id("gpui-fps-hud")
            .absolute()
            .left(position.x)
            .top(position.y)
            .flex()
            .flex_col()
            .w(current_size.width)
            .h(current_size.height)
            .px_2()
            .py(px(3.0))
            .gap_0p5()
            .rounded(px(8.0))
            .shadow_lg()
            .font_family(DEFAULT_FONT)
            .text_size(TEXT_SIZE)
            .text_color(style.muted)
            .cursor_default()
            .overflow_hidden()
            .occlude()
            .child(backdrop_blur(
                card().opacity(0.5).into(),
                px(20.0),
                px(8.0),
                0.012,
            ))
            .child(capture)
            .child(top_bar)
            .when(show_details, |hud| {
                hud.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .opacity(details_opacity)
                        .child(self.render_chart_panel(style))
                        .child(reading(
                            &t("fps.fps"),
                            format!("{fps:.0}"),
                            fps_color,
                            style,
                        ))
                        .child(reading(
                            &t("fps.frame"),
                            rust_i18n::t!("fps.unit_ms", value = format!("{frame_millis:.1}"))
                                .to_string(),
                            style.foreground,
                            style,
                        ))
                        .child(reading(
                            &t("fps.drop"),
                            format!("{dropped:.1}%"),
                            style.level_color(if dropped > 0.0 { 1.0 } else { 0.0 }, 0.5),
                            style,
                        ))
                        .child(reading(
                            &t("fps.cpu"),
                            resources
                                .map(|sample| format!("{:.1}%", sample.cpu_percent))
                                .unwrap_or_else(|| "--".to_string()),
                            style.foreground,
                            style,
                        ))
                        .child(reading(
                            &t("fps.memory"),
                            resources
                                .map(|sample| format_bytes(sample.memory_bytes))
                                .unwrap_or_else(|| "--".to_string()),
                            style.foreground,
                            style,
                        )),
                )
            })
            .child(dashed_outline(accent().opacity(0.7).into()))
    }
}

fn fps_color(fps: f32, budget: Duration, style: FpsStyle) -> Hsla {
    if fps <= 0.0 {
        return style.muted;
    }

    let target = 1.0 / budget.as_secs_f32();
    if fps >= target * FPS_TOLERANCE {
        style.good
    } else if fps >= target * 0.5 {
        style.warn
    } else {
        style.bad
    }
}

fn reading(label: &str, value: String, value_color: Hsla, style: FpsStyle) -> Div {
    div()
        .flex()
        .w_full()
        .justify_between()
        .gap_2()
        .py(px(1.0))
        .child(div().text_color(style.muted).child(label.to_string()))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(value_color)
                .child(value),
        )
}

fn hud_morph_size(collapse_t: f32) -> Size<Pixels> {
    size(
        HUD_WIDTH + (HUD_COLLAPSED_WIDTH - HUD_WIDTH) * collapse_t,
        HUD_HEIGHT + (HUD_COLLAPSED_HEIGHT - HUD_HEIGHT) * collapse_t,
    )
}

fn default_hud_position(viewport: Size<Pixels>) -> Point<Pixels> {
    clamp_hud_position(
        point(viewport.width - HUD_WIDTH - HUD_MARGIN, HUD_TOP),
        viewport,
        size(HUD_WIDTH, HUD_HEIGHT),
    )
}

fn clamp_hud_position(
    position: Point<Pixels>,
    viewport: Size<Pixels>,
    hud_size: Size<Pixels>,
) -> Point<Pixels> {
    let max_x = (viewport.width - hud_size.width - HUD_MARGIN).max(HUD_MARGIN);
    let max_y = (viewport.height - hud_size.height - HUD_MARGIN).max(HUD_TOP);
    point(
        position.x.clamp(HUD_MARGIN, max_x),
        position.y.clamp(HUD_TOP, max_y),
    )
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let bytes = bytes as f64;
    if bytes >= GIB {
        let val = format!("{:.2}", bytes / GIB);
        rust_i18n::t!("fps.unit_gb", value = val).to_string()
    } else {
        let val = format!("{:.0}", bytes / MIB);
        rust_i18n::t!("fps.unit_mb", value = val).to_string()
    }
}

#[derive(IntoElement)]
pub struct FpsOverlay {
    monitor: Entity<FpsMonitor>,
}

impl FpsOverlay {
    pub fn new(monitor: &Entity<FpsMonitor>) -> Self {
        Self {
            monitor: monitor.clone(),
        }
    }
}

impl RenderOnce for FpsOverlay {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.monitor
    }
}

#[derive(Default)]
struct Monitors(HashMap<WindowId, Entity<FpsMonitor>>);

impl Global for Monitors {}

pub fn fps_monitor(window: &mut Window, cx: &mut App) -> FpsOverlay {
    let window_id = window.window_handle().window_id();
    let existing = cx
        .try_global::<Monitors>()
        .and_then(|state| state.0.get(&window_id).cloned());
    let monitor = match existing {
        Some(monitor) => monitor,
        None => {
            let monitor = cx.new(|cx| FpsMonitor::new(window, cx));
            cx.default_global::<Monitors>()
                .0
                .insert(window_id, monitor.clone());
            monitor
        }
    };

    FpsOverlay::new(&monitor)
}

#[cfg(test)]
mod tests {
    use gpui::{point, px, size};

    use super::{HUD_HEIGHT, HUD_MARGIN, HUD_WIDTH, clamp_hud_position};

    #[test]
    fn hud_position_stays_inside_viewport() {
        let viewport = size(px(900.0), px(700.0));
        let hud_size = size(HUD_WIDTH, HUD_HEIGHT);
        assert_eq!(
            clamp_hud_position(point(px(-20.0), px(900.0)), viewport, hud_size),
            point(HUD_MARGIN, px(530.0))
        );
    }
}
