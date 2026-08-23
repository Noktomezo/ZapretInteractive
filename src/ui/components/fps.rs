use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::*;

use crate::ui::components::backdrop_blur::backdrop_blur;
use crate::ui::components::dashed_outline::dashed_outline;
use crate::ui::foundation::colors::{accent, card};

const DEFAULT_FRAME_BUDGET: Duration = Duration::from_nanos(16_666_667); // 60Hz
const DEFAULT_CAPACITY: usize = 120;
const DEFAULT_RESOURCE_INTERVAL: Duration = Duration::from_millis(500);
const AXIS_DECAY: f32 = 0.04;
const HUD_WIDTH: Pixels = px(208.0);
const HUD_HEIGHT: Pixels = px(154.0);
const HUD_MARGIN: Pixels = px(16.0);
const HUD_TOP: Pixels = px(56.0);
const TEXT_SIZE: Pixels = px(10.0);
const TRACE_OPACITY: f32 = 0.35;
const CHART_HEIGHT: Pixels = px(42.0);
const READOUT_INTERVAL: Duration = Duration::from_millis(500);
const FPS_TOLERANCE: f32 = 0.95;
const FPS_WINDOW: Duration = Duration::from_secs(1);
const DEFAULT_FONT: &str = "IBM Plex Mono";

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
            background: hsla(0.0, 0.0, 0.04, 0.92),
            foreground: hsla(0.0, 0.0, 0.98, 1.0),
            muted: hsla(0.0, 0.0, 0.62, 1.0),
            good: hsla(0.41, 0.95, 0.56, 1.0),
            warn: hsla(0.11, 0.95, 0.60, 1.0),
            bad: hsla(0.99, 0.90, 0.62, 1.0),
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
}

impl FrameSampler {
    pub(crate) fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            samples: VecDeque::with_capacity(capacity),
            frame_times: VecDeque::new(),
            capacity,
            last_tick: None,
        }
    }

    pub(crate) fn tick(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_tick {
            let draw = now.duration_since(last);
            if self.samples.len() == self.capacity {
                self.samples.pop_front();
            }
            self.samples.push_back(FrameSample { draw });
            self.frame_times.push_back(now);
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
    style: FpsStyle,
    frame_budget: Duration,
    continuous: bool,
    show_resources: bool,
    resource_interval: Duration,
    resources: Option<ResourceSample>,
    axis_max: f32,
    resource_task: Option<Task<()>>,
    position: Point<Pixels>,
    drag: Option<HudDrag>,
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
            style: FpsStyle::default(),
            frame_budget,
            continuous: true,
            show_resources: true,
            resource_interval: DEFAULT_RESOURCE_INTERVAL,
            resources: None,
            axis_max: frame_budget.as_secs_f32() * 2.0,
            resource_task: None,
            position: default_hud_position(window.viewport_size()),
            drag: None,
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

    fn render_chart(&self) -> impl IntoElement {
        let style = self.style;
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
                    let mut path = PathBuilder::stroke(px(1.0));
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

    fn render_chart_panel(&self) -> Div {
        div()
            .relative()
            .overflow_hidden()
            .w_full()
            .h(CHART_HEIGHT)
            .rounded(px(4.0))
            .bg(self.style.background.opacity(0.55))
            .child(self.render_chart())
    }
}

impl Render for FpsMonitor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sampler.tick();
        self.update_readout();
        self.update_axis();
        self.start_resource_sampling(cx);
        if self.continuous {
            window.request_animation_frame();
        }

        let style = self.style;
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
        let capture = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                window.on_mouse_event({
                    let monitor = capture_monitor.clone();
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase != DispatchPhase::Capture || !event.dragging() {
                            return;
                        }
                        let drag = monitor.read_with(cx, |monitor, _| monitor.drag);
                        let Some(drag) = drag else { return };
                        let next = point(
                            drag.origin.x + event.position.x - drag.pointer.x,
                            drag.origin.y + event.position.y - drag.pointer.y,
                        );
                        let position = clamp_hud_position(next, window.viewport_size());
                        monitor.update(cx, |monitor, cx| {
                            monitor.position = position;
                            cx.notify();
                        });
                        cx.stop_propagation();
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
                        cx.stop_propagation();
                        window.refresh();
                    }
                });
            },
        )
        .absolute()
        .inset_0();

        div()
            .id("gpui-fps-hud")
            .absolute()
            .left(position.x)
            .top(position.y)
            .flex()
            .flex_col()
            .w(HUD_WIDTH)
            .h(HUD_HEIGHT)
            .px_2()
            .py_1p5()
            .gap_0p5()
            .rounded(px(8.0))
            .shadow_lg()
            .font_family(DEFAULT_FONT)
            .text_size(TEXT_SIZE)
            .text_color(style.muted)
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.drag = Some(HudDrag {
                        pointer: event.position,
                        origin: this.position,
                    });
                    cx.notify();
                    cx.stop_propagation();
                    window.refresh();
                }),
            )
            .child(backdrop_blur(
                card().opacity(0.5).into(),
                px(20.0),
                px(8.0),
                0.012,
            ))
            .child(capture)
            .child(
                div()
                    .flex()
                    .w_full()
                    .justify_between()
                    .text_color(style.foreground)
                    .child("PERFORMANCE")
                    .child(if dragging { "MOVING" } else { "DRAG" }),
            )
            .child(self.render_chart_panel())
            .child(reading("FPS", format!("{fps:.0}"), fps_color, style))
            .child(reading(
                "FRAME",
                format!("{frame_millis:.1} ms"),
                style.foreground,
                style,
            ))
            .child(reading(
                "DROP",
                format!("{dropped:.1}%"),
                style.level_color(if dropped > 0.0 { 1.0 } else { 0.0 }, 0.5),
                style,
            ))
            .child(reading(
                "CPU",
                resources
                    .map(|sample| format!("{:.1}%", sample.cpu_percent))
                    .unwrap_or_else(|| "--".to_string()),
                style.foreground,
                style,
            ))
            .child(reading(
                "MEMORY",
                resources
                    .map(|sample| format_bytes(sample.memory_bytes))
                    .unwrap_or_else(|| "--".to_string()),
                style.foreground,
                style,
            ))
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

fn reading(label: &'static str, value: String, value_color: Hsla, style: FpsStyle) -> Div {
    div()
        .flex()
        .w_full()
        .justify_between()
        .gap_2()
        .py(px(1.0))
        .child(div().text_color(style.muted).child(label))
        .child(div().text_color(value_color).child(value))
}

fn default_hud_position(viewport: Size<Pixels>) -> Point<Pixels> {
    clamp_hud_position(
        point(viewport.width - HUD_WIDTH - HUD_MARGIN, HUD_TOP),
        viewport,
    )
}

fn clamp_hud_position(position: Point<Pixels>, viewport: Size<Pixels>) -> Point<Pixels> {
    let max_x = (viewport.width - HUD_WIDTH - HUD_MARGIN).max(HUD_MARGIN);
    let max_y = (viewport.height - HUD_HEIGHT - HUD_MARGIN).max(HUD_TOP);
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
        format!("{:.2} GB", bytes / GIB)
    } else {
        format!("{:.0} MB", bytes / MIB)
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

    use super::{HUD_MARGIN, clamp_hud_position};

    #[test]
    fn hud_position_stays_inside_viewport() {
        let viewport = size(px(900.0), px(700.0));
        assert_eq!(
            clamp_hud_position(point(px(-20.0), px(900.0)), viewport),
            point(HUD_MARGIN, px(530.0))
        );
    }
}
