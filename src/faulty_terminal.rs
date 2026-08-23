use std::time::Instant;

use gpui::prelude::*;
use gpui::*;

use crate::domain::ConnectionStatus;
use crate::ui::foundation::colors;

const TITLEBAR_HEIGHT: f32 = 40.0;
const REFERENCE_CANVAS_HEIGHT: f32 = 700.0;
const SHADER_SCALE: f32 = 1.5;
const STYLE_DAMPING: f32 = 0.1;
const FAULTY_TERMINAL_SHADER: GpuCanvasShader = GpuCanvasShader::new(
    "faulty_terminal",
    include_str!("shaders/faulty_terminal.hlsl"),
);

#[derive(Clone, Copy, PartialEq)]
struct TerminalStyle {
    tint: [f32; 4],
    background: [f32; 4],
    flicker: f32,
    curvature: f32,
    scanline: f32,
}

#[inline(always)]
fn shader_color(color: Rgba) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

impl TerminalStyle {
    fn for_status(status: ConnectionStatus, dark: bool) -> Self {
        let tint = match (status, dark) {
            (ConnectionStatus::Connected, true) => shader_color(colors::green_400()),
            (ConnectionStatus::Connected, false) => shader_color(colors::green_600()),
            (ConnectionStatus::Connecting | ConnectionStatus::Disconnecting, true) => {
                shader_color(colors::yellow_400())
            }
            (ConnectionStatus::Connecting | ConnectionStatus::Disconnecting, false) => {
                shader_color(colors::yellow_600())
            }
            (ConnectionStatus::Disconnected | ConnectionStatus::Error, true) => {
                shader_color(colors::red_400())
            }
            (ConnectionStatus::Disconnected | ConnectionStatus::Error, false) => {
                shader_color(colors::red_600())
            }
        };
        Self {
            tint,
            background: if dark {
                shader_color(colors::black())
            } else {
                shader_color(colors::paper())
            },
            flicker: f32::from(!matches!(status, ConnectionStatus::Connected)),
            curvature: f32::from(!matches!(
                status,
                ConnectionStatus::Connected | ConnectionStatus::Disconnecting
            )) * 0.1,
            scanline: f32::from(matches!(status, ConnectionStatus::Disconnected)) * 0.22,
        }
    }

    fn approach(&mut self, target: Self, damping: f32) {
        for (current, target) in self.tint.iter_mut().zip(target.tint) {
            *current += (target - *current) * damping;
        }
        for (current, target) in self.background.iter_mut().zip(target.background) {
            *current += (target - *current) * damping;
        }
        self.curvature += (target.curvature - self.curvature) * damping;
        self.scanline += (target.scanline - self.scanline) * damping;
        self.flicker = target.flicker;
    }
}

pub struct FaultyTerminal {
    active: bool,
    status: ConnectionStatus,
    dark: bool,
    started_at: Instant,
    mouse: [f32; 2],
    canvas_size: [f32; 2],
    style: TerminalStyle,
    target_style: TerminalStyle,
}

impl FaultyTerminal {
    pub fn new(window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let status = ConnectionStatus::Disconnected;
        let dark = crate::ui::foundation::colors::is_dark();
        let style = TerminalStyle::for_status(status, dark);
        Self {
            active: true,
            status,
            dark,
            started_at: Instant::now(),
            mouse: [0.5, 0.5],
            canvas_size: canvas_size(window.viewport_size()),
            style,
            target_style: style,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
        self.update_target_style();
    }

    pub fn set_dark_theme(&mut self, dark: bool) {
        if self.dark == dark {
            return;
        }
        self.dark = dark;
        self.update_target_style();
    }

    fn update_target_style(&mut self) {
        let target = TerminalStyle::for_status(self.status, self.dark);
        if self.target_style == target {
            return;
        }
        self.target_style = target;
        if !self.active {
            self.style = target;
        }
    }

    pub fn set_mouse(&mut self, position: Point<Pixels>) {
        self.mouse = [
            (f32::from(position.x) / self.canvas_size[0]).clamp(0.0, 1.0),
            (1.0 - (f32::from(position.y) - TITLEBAR_HEIGHT) / self.canvas_size[1]).clamp(0.0, 1.0),
        ];
    }
}

impl Render for FaultyTerminal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.canvas_size = canvas_size(window.viewport_size());
        if self.active {
            cx.on_next_frame(window, |this, _, cx| {
                this.style.approach(this.target_style, STYLE_DAMPING);
                cx.notify();
            });
        }

        let elapsed = self.started_at.elapsed().as_secs_f32();
        let canvas_size = self.canvas_size;
        let world_size = [
            canvas_size[0] / REFERENCE_CANVAS_HEIGHT * SHADER_SCALE,
            canvas_size[1] / REFERENCE_CANVAS_HEIGHT * SHADER_SCALE,
        ];
        let style = self.style;
        let mouse = self.mouse;
        let mut uniform_slots = [[0.0; 4]; GPU_CANVAS_UNIFORM_SLOTS];
        uniform_slots[0] = [
            elapsed * 0.1,
            (elapsed / 2.0).min(1.0),
            style.curvature,
            style.scanline,
        ];
        uniform_slots[1] = [mouse[0], mouse[1], world_size[0], world_size[1]];
        uniform_slots[2] = style.tint;
        uniform_slots[3] = style.background;
        uniform_slots[4][0] = style.flicker;
        let uniforms = GpuCanvasUniforms::new(uniform_slots);

        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                window.paint_gpu_canvas(PaintGpuCanvas {
                    bounds,
                    shader: FAULTY_TERMINAL_SHADER,
                    uniforms,
                    input: GpuCanvasInput::None,
                    opacity: 0.9,
                });
            },
        )
        .absolute()
        .top_0()
        .right_0()
        .w(px(canvas_size[0]))
        .h(px(canvas_size[1]))
    }
}

fn canvas_size(viewport: Size<Pixels>) -> [f32; 2] {
    [
        f32::from(viewport.width).max(1.0),
        (f32::from(viewport.height) - TITLEBAR_HEIGHT).max(1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::TerminalStyle;
    use crate::domain::ConnectionStatus;

    #[test]
    fn style_damping_moves_toward_target_without_overshooting() {
        let mut current = TerminalStyle::for_status(ConnectionStatus::Disconnected, true);
        let target = TerminalStyle::for_status(ConnectionStatus::Connected, true);

        current.approach(target, 0.1);

        assert!(current.tint[0] < 0.8510 && current.tint[0] > target.tint[0]);
        assert!(current.curvature < 0.1 && current.curvature > target.curvature);
        assert_eq!(current.flicker, target.flicker);
    }

    #[test]
    fn light_theme_uses_a_light_canvas() {
        let style = TerminalStyle::for_status(ConnectionStatus::Disconnected, false);
        assert!(style.background[0] > 0.9);
        assert!(style.tint[0] < style.background[0]);
    }

    #[test]
    fn shader_exposes_the_gpu_canvas_entry_points() {
        let shader = include_str!("shaders/faulty_terminal.hlsl");
        assert!(shader.contains("gpu_canvas_vertex"));
        assert!(shader.contains("gpu_canvas_fragment"));
    }
}
