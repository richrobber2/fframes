//! A field of 2,304 dots, every one computed in Rust on every frame: a wave runs across the grid,
//! a lens pushes dots away from the pointer and clicks send out ripples. It is the heavy example
//! for the live runtime (`fframes-live`) and still renders to a video file: without a pointer the
//! lens flies on its own and scripted ripples go off.

use std::sync::{Mutex, PoisonError};

use fframes::{
    AudioMap, Color, Duration, FFramesContext, Frame, Input, InteractionEvent, Svgr, Video,
    include_media_dir,
};

include_media_dir!(pub struct DotFieldMedia, "examples/dot-field/media");

pub const COLUMNS: usize = 64;
pub const ROWS: usize = 36;
/// Number of dots drawn every frame.
pub const DOTS: usize = COLUMNS * ROWS;
const SPACING: f32 = 30.0;

const LENS_RADIUS: f32 = 300.0;
const LENS_PUSH: f32 = 48.0;
const RIPPLE_SPEED: f32 = 900.0;
const RIPPLE_LIFE: f32 = 1.8;
const RIPPLE_WIDTH: f32 = 42.0;
const RIPPLE_PUSH: f32 = 26.0;
const MAX_RIPPLES: usize = 8;

/// Ripples that are part of the video itself: (second, x, y).
const SCRIPTED_RIPPLES: [(f32, f32, f32); 3] = [
    (1.5, 520.0, 420.0),
    (5.0, 1420.0, 640.0),
    (8.5, 960.0, 300.0),
];

const DEEP: [f32; 3] = [30.0, 41.0, 82.0];
const BLUE: [f32; 3] = [56.0, 132.0, 255.0];
const ORANGE: [f32; 3] = [251.0, 106.0, 34.0];

#[derive(Debug, Clone, Copy)]
struct Ripple {
    x: f32,
    y: f32,
    /// `Input::time` of the click.
    start: f32,
}

#[derive(Debug)]
pub struct DotField<'a> {
    pub media: &'a DotFieldMedia,
    /// Ripples started by clicks in the live runtime, newest last.
    ripples: Mutex<Vec<Ripple>>,
}

impl<'a> DotField<'a> {
    pub fn new(media: &'a DotFieldMedia) -> Self {
        Self {
            media,
            ripples: Mutex::new(Vec::new()),
        }
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// One decimal is plenty for 1080p and keeps the frame markup short.
fn round1(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

/// Radii in quarter pixels and colors in steps of 4: the eye cannot tell, and in the live runtime a
/// dot whose rounded values did not change costs no DOM update that frame.
fn quarter(value: f32) -> f32 {
    (value * 4.0).round() / 4.0
}

fn channel(value: f32) -> u8 {
    ((value / 4.0).round() * 4.0).min(255.0) as u8
}

/// Where the lens is: under the pointer, or flying a Lissajous path when there is none.
fn lens_position(input: &Input, seconds: f32) -> (f32, f32) {
    input.pointer.unwrap_or_else(|| {
        (
            960.0 + 620.0 * (seconds * 0.55).sin(),
            540.0 + 300.0 * (seconds * 0.9 + 0.6).sin(),
        )
    })
}

impl Video for DotField<'_> {
    const FPS: usize = 30;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;
    const BACKGROUND_COLOR: Color = Color::hex("#0b0f17");

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(12.0)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn on_event(&self, event: &InteractionEvent, input: &Input) {
        if let InteractionEvent::PointerDown { x, y } = *event {
            let mut ripples = self.ripples.lock().unwrap_or_else(PoisonError::into_inner);
            if ripples.len() == MAX_RIPPLES {
                ripples.remove(0);
            }
            ripples.push(Ripple {
                x,
                y,
                start: input.time,
            });
        }
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let input = ctx.input();
        let seconds = frame.seconds();
        let (lens_x, lens_y) = lens_position(input, seconds);
        let lens_strength = if input.pointer_down { 1.35 } else { 1.0 };

        // (x, y, age) of every ripple still spreading.
        let mut ripples: Vec<(f32, f32, f32)> = SCRIPTED_RIPPLES
            .iter()
            .map(|&(at, x, y)| (x, y, seconds - at))
            .collect();
        ripples.extend(
            self.ripples
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .iter()
                .map(|ripple| (ripple.x, ripple.y, input.time - ripple.start)),
        );
        ripples.retain(|&(_, _, age)| (0.0..RIPPLE_LIFE).contains(&age));

        let dots = (0..DOTS)
            .map(|index| {
                let (column, row) = (index % COLUMNS, index / COLUMNS);
                let base_x = SPACING * column as f32 + SPACING / 2.0;
                let base_y = SPACING * row as f32 + SPACING / 2.0;
                let (mut x, mut y) = (base_x, base_y);

                let from_center = (base_x - 960.0).hypot(base_y - 540.0);
                let wave = 0.5 + 0.5 * (from_center * 0.016 - seconds * 2.4).sin();

                let (dx, dy) = (base_x - lens_x, base_y - lens_y);
                let distance = dx.hypot(dy).max(0.001);
                let lens = (1.0 - distance / LENS_RADIUS).max(0.0);
                let push = lens * lens * LENS_PUSH * lens_strength;
                x += dx / distance * push;
                y += dy / distance * push;

                let mut ring = 0.0;
                for &(rx, ry, age) in &ripples {
                    let (dx, dy) = (base_x - rx, base_y - ry);
                    let distance = dx.hypot(dy).max(0.001);
                    let band = (-((distance - age * RIPPLE_SPEED) / RIPPLE_WIDTH).powi(2)).exp()
                        * (1.0 - age / RIPPLE_LIFE);
                    x += dx / distance * band * RIPPLE_PUSH;
                    y += dy / distance * band * RIPPLE_PUSH;
                    ring += band;
                }

                let energy = (wave * 0.45 + lens * 0.9 + ring).min(1.0);
                let radius = (2.0 + 5.0 * wave + 7.0 * lens + 6.0 * ring).min(13.0);
                let color = if energy < 0.5 {
                    mix(DEEP, BLUE, energy * 2.0)
                } else {
                    mix(BLUE, ORANGE, energy * 2.0 - 1.0)
                };
                let fill = format!(
                    "rgb({},{},{})",
                    channel(color[0]),
                    channel(color[1]),
                    channel(color[2])
                );

                fframes::svgr!(
                    <circle cx={round1(x)} cy={round1(y)} r={quarter(radius)} fill={fill} />
                )
            })
            .collect::<Vec<_>>();

        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080"
                 width={Self::WIDTH} height={Self::HEIGHT}>
                <rect width="1920" height="1080" fill="#0b0f17" />
                <g>{dots}</g>
                <text x="48" y="1032" font-family="DM Sans" font-size="30" fill="#f8fafc"
                      fill-opacity="0.85" pointer-events="none">
                    "2,304 dots, every one computed in Rust each frame"
                </text>
            </svg>
        )
    }
}
