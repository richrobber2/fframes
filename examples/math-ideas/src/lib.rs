//! Three big ideas of mathematics as moving pictures you can play with: complex multiplication is
//! rotation (Euler's formula), a matrix moves the whole plane (determinant and eigenvectors), and a
//! square wave is a sum of circles (Fourier series and the Gibbs overshoot).
//!
//! It renders as a 36-second explainer video, where the parameters move on their own, and runs live
//! in the browser through `fframes-live`, where the viewer sets them: the pointer picks the angle
//! and the matrix, the + and − buttons the number of circles. Every number on screen comes from
//! [`math`], whose tests state each idea as a fact.

pub mod math;

use std::f32::consts::{PI, TAU};

use fframes::{
    AudioMap, Color, Duration, FFramesContext, Frame, Input, Scene, Scenes, Svgr, Video,
    animation::Easing, include_media_dir,
};

use crate::math::Matrix2;

include_media_dir!(pub struct MathIdeasMedia, "examples/math-ideas/media");

const INK: &str = "#0d0d0d";
const MUTED: &str = "#5d5d5d";
const FAINT: &str = "#8f8f8f";
const PAPER: &str = "#fbfaf7";
const BLUE: &str = "#0285ff";
const ORANGE: &str = "#fb6a22";
const GREEN: &str = "#16a34a";

/// Everything left of this x is the picture; the panel with the words starts here.
const PANEL_X: f32 = 1180.0;
const SCENE_SECONDS: f32 = 12.0;

/// Numbers as a reader writes them: a real minus sign, no "-0.00".
fn num(value: f32, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    let zero = format!("{:.decimals$}", 0.0);
    if text.trim_start_matches('-') == zero {
        zero
    } else {
        text.replace('-', "−")
    }
}

/// The pointer, when it is over the picture.
fn pointer_on_picture(input: &Input) -> Option<(f32, f32)> {
    input.pointer.filter(|&(x, _)| x < PANEL_X)
}

/// The text panel shared by the scenes: a fixed number of lines so the live frame keeps its shape.
fn panel<'a>(
    frame: &Frame,
    index: usize,
    title: &'static str,
    body: [&'static str; 4],
    facts: [String; 5],
    hint: &'static str,
) -> Svgr<'a> {
    let reveal = frame.animate(&fframes::timeline!(
        at 0.0, animate 0.0_f32 => 1.0, Easing::Spring { mass: 1.0, stiffness: 170.0, damping: 26.0 }
    ));
    let label = format!("IDEA {} OF 3", index + 1);
    let x = PANEL_X + 40.0;
    let body = body
        .iter()
        .enumerate()
        .map(|(line, text)| {
            let y = 300.0 + 48.0 * line as f32;
            fframes::svgr!(
                <text data-role="body" x={x} y={y} font-family="DM Sans" font-size="33" fill={MUTED}>{*text}</text>
            )
        })
        .collect::<Vec<_>>();
    let facts = facts
        .into_iter()
        .enumerate()
        // Browsers collapse runs of spaces in SVG text; no-break spaces keep the columns aligned.
        .map(|(line, text)| (line, text.replace(' ', "\u{a0}")))
        .map(|(line, text)| {
            let y = 560.0 + 52.0 * line as f32;
            fframes::svgr!(
                <text data-role="fact" x={x} y={y} font-family="JetBrains Mono" font-size="30" fill={INK}>{text}</text>
            )
        })
        .collect::<Vec<_>>();

    fframes::svgr!(
        <g opacity={reveal} transform={format!("translate({} 0)", (1.0 - reveal) * 30.0)}>
            <text data-role="label" x={x} y="150" font-family="DM Sans" font-size="24" fill={ORANGE} letter-spacing="3">{label}</text>
            <text data-role="title" x={x} y="215" font-family="DM Sans" font-size="54" fill={INK}>{title}</text>
            <g>{body}</g>
            <line x1={x} y1="500" x2={x + 120.0} y2="500" stroke={INK} stroke-width="2" />
            <g>{facts}</g>
            <text data-role="hint" x={x} y="1010" font-family="DM Sans" font-size="26" fill={FAINT}>{hint}</text>
        </g>
    )
}

/// Idea 1: multiplying by `e^{iθ}` turns a point by `θ`; its shadows are `cos θ` and `sin θ`.
#[derive(Debug)]
pub struct EulerScene;

impl Scene for EulerScene {
    fn name(&self) -> &'static str {
        "Rotation"
    }

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(SCENE_SECONDS)
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        const CX: f32 = 580.0;
        const CY: f32 = 540.0;
        const R: f32 = 330.0;

        // The pointer over the picture, else a typed value (`theta`), else the autopilot.
        let input = ctx.input();
        let theta = match pointer_on_picture(input) {
            Some((px, py)) => (CY - py).atan2(px - CX).rem_euclid(TAU),
            None => input
                .value("theta")
                .and_then(math::parse_angle)
                .unwrap_or(0.62 * frame.seconds())
                .rem_euclid(TAU),
        };
        let (cos, sin) = math::euler(theta);
        let (px, py) = (CX + R * cos, CY - R * sin);

        // The angle as an arc, counterclockwise from the positive real axis.
        let arc_r = 70.0;
        let arc = format!(
            "M {} {} A {arc_r} {arc_r} 0 {} 0 {} {}",
            CX + arc_r,
            CY,
            u8::from(theta > PI),
            CX + arc_r * cos,
            CY - arc_r * sin
        );
        let near_pi = (theta - PI).abs() < 0.08;
        // Labels sit on the side away from the circle's center.
        let label_x = if cos >= 0.0 { px + 22.0 } else { px - 150.0 };
        let cos_label_y = if sin >= 0.0 { CY + 50.0 } else { CY - 26.0 };
        // Clear of the angle arc, which hugs the center on the side of the point.
        let sin_label_x = if cos >= 0.0 { CX - 130.0 } else { CX + 90.0 };

        let facts = [
            format!(
                "θ = {} rad = {}°",
                num(theta, 2),
                num(theta.to_degrees(), 0)
            ),
            "e^(iθ) = cos θ + i·sin θ".to_string(),
            format!("       = {} + {}i", num(cos, 2), num(sin, 2)).replace("+ −", "− "),
            format!("|e^(iθ)| = {}", num(cos.hypot(sin), 2)),
            if near_pi {
                "θ = π:  e^(iπ) = −1".to_string()
            } else {
                "turn to π and find −1".to_string()
            },
        ];

        fframes::svgr!(
            <g>
                <line x1="200" y1="540" x2="960" y2="540" stroke="#d6d3cc" stroke-width="2" />
                <line x1="580" y1="160" x2="580" y2="920" stroke="#d6d3cc" stroke-width="2" />
                <circle cx="580" cy="540" r="330" fill="none" stroke={INK} stroke-width="3" />
                <text x="925" y="580" font-family="JetBrains Mono" font-size="28" fill={FAINT}>"1"</text>
                <text x="190" y="580" font-family="JetBrains Mono" font-size="28" fill={FAINT}>"−1"</text>
                <text x="600" y="200" font-family="JetBrains Mono" font-size="28" fill={FAINT}>"i"</text>
                <text x="600" y="905" font-family="JetBrains Mono" font-size="28" fill={FAINT}>"−i"</text>

                <path d={arc} fill="none" stroke={ORANGE} stroke-width="4" />
                <line x1={px} y1={py} x2={px} y2={CY} stroke={BLUE} stroke-width="2" stroke-dasharray="8 8" />
                <line x1={px} y1={py} x2={CX} y2={py} stroke={ORANGE} stroke-width="2" stroke-dasharray="8 8" />
                <line x1={CX} y1={CY} x2={px} y2={CY} stroke={BLUE} stroke-width="10" stroke-linecap="round" />
                <line x1={CX} y1={CY} x2={CX} y2={py} stroke={ORANGE} stroke-width="10" stroke-linecap="round" />
                <line x1={CX} y1={CY} x2={px} y2={py} stroke={INK} stroke-width="4" />
                <circle cx={px} cy={py} r="14" fill={INK} />
                <text x={label_x} y={py - 24.0} font-family="JetBrains Mono" font-size="30" fill={INK}>"e^(iθ)"</text>
                <text x={CX.midpoint(px) - 40.0} y={cos_label_y}
                      font-family="JetBrains Mono" font-size="28" fill={BLUE}>"cos θ"</text>
                <text x={sin_label_x} y={CY.midpoint(py)} font-family="JetBrains Mono" font-size="28" fill={ORANGE}>"sin θ"</text>

                {panel(&frame, 0, "Rotation is multiplication", [
                    "Multiplying by a unit complex",
                    "number turns a point around zero",
                    "and keeps its size. Its shadows on",
                    "the axes are the cosine and sine.",
                ], facts, "Live: point at the picture to set the angle.")}
            </g>
        )
    }
}

/// Idea 2: a matrix sends î and ĵ somewhere and the grid follows. The determinant is the area of
/// the unit square's image; eigenvectors are the directions that only stretch.
#[derive(Debug)]
pub struct MatrixScene;

impl MatrixScene {
    const OX: f32 = 580.0;
    const OY: f32 = 540.0;
    const UNIT: f32 = 100.0;
    const J: (f32, f32) = (0.3, 1.1);

    fn to_screen(v: (f32, f32)) -> (f32, f32) {
        (Self::OX + Self::UNIT * v.0, Self::OY - Self::UNIT * v.1)
    }
}

impl Scene for MatrixScene {
    fn name(&self) -> &'static str {
        "Matrices"
    }

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(SCENE_SECONDS)
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        // î: the pointer over the picture, else a typed value (`i`), else the autopilot.
        // ĵ: a typed value (`j`), else fixed.
        let input = ctx.input();
        let clamp = |(x, y): (f32, f32)| (x.clamp(-4.0, 4.0), y.clamp(-4.0, 4.0));
        let i = if let Some((px, py)) = pointer_on_picture(input) {
            clamp(((px - Self::OX) / Self::UNIT, (Self::OY - py) / Self::UNIT))
        } else if let Some(i) = input.value("i").and_then(math::parse_vector) {
            clamp(i)
        } else {
            let a = 0.52 * frame.seconds();
            (1.4 * a.cos(), 0.9 * a.sin())
        };
        let j = input
            .value("j")
            .and_then(math::parse_vector)
            .map_or(Self::J, clamp);
        let m = Matrix2 { i, j };
        let det = m.det();
        let eigen = m.eigen();

        let grid = (-8..=8)
            .flat_map(|k| {
                let k = k as f32;
                [((k, -8.0), (k, 8.0)), ((-8.0, k), (8.0, k))]
            })
            .map(|(from, to)| {
                let (x1, y1) = Self::to_screen(m.apply(from));
                let (x2, y2) = Self::to_screen(m.apply(to));
                fframes::svgr!(<line x1={x1} y1={y1} x2={x2} y2={y2} stroke="#a9c4e8" stroke-width="2" />)
            })
            .collect::<Vec<_>>();

        let square = [(0.0, 0.0), m.i, (m.i.0 + m.j.0, m.i.1 + m.j.1), m.j]
            .map(Self::to_screen)
            .iter()
            .map(|(x, y)| format!("{x},{y}"))
            .collect::<Vec<_>>()
            .join(" ");
        let square_fill = if det >= 0.0 { ORANGE } else { "#7c3aed" };

        let eigen_lines = (0..2)
            .map(|index| {
                let (lambda, v, visible) = match eigen.get(index) {
                    Some(&(lambda, v)) => (lambda, v, 1),
                    None => (0.0, (1.0, 0.0), 0),
                };
                let (x1, y1) = Self::to_screen((-9.0 * v.0, -9.0 * v.1));
                let (x2, y2) = Self::to_screen((9.0 * v.0, 9.0 * v.1));
                let (lx, ly) = Self::to_screen((3.6 * v.0, 3.6 * v.1));
                fframes::svgr!(
                    <g opacity={visible}>
                        <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={GREEN} stroke-width="4" stroke-dasharray="14 10" />
                        <text x={lx + 12.0} y={ly - 12.0} font-family="JetBrains Mono" font-size="28" fill={GREEN}>
                            {format!("λ = {}", num(lambda, 2))}
                        </text>
                    </g>
                )
            })
            .collect::<Vec<_>>();

        let (ix, iy) = Self::to_screen(m.i);
        let (jx, jy) = Self::to_screen(m.j);

        let facts = [
            format!("M = [ {:>5}  {:>5} ]", num(m.i.0, 2), num(m.j.0, 2)),
            format!("    [ {:>5}  {:>5} ]", num(m.i.1, 2), num(m.j.1, 2)),
            if det < 0.0 {
                format!("det M = {}: space flips", num(det, 2))
            } else {
                format!("det M = {}: area × {}", num(det, 2), num(det, 2))
            },
            match eigen.as_slice() {
                [(a, _), (b, _)] => format!("λ = {}, {}", num(*a, 2), num(*b, 2)),
                _ => "λ complex: it turns".to_string(),
            },
            format!("λ₁·λ₂ = det, λ₁+λ₂ = {}", num(m.trace(), 2)),
        ];

        fframes::svgr!(
            <g>
                <g stroke="#ebe8e1" stroke-width="2">
                    <line x1="80" y1="140" x2="1080" y2="140" />
                    <line x1="80" y1="240" x2="1080" y2="240" />
                    <line x1="80" y1="340" x2="1080" y2="340" />
                    <line x1="80" y1="440" x2="1080" y2="440" />
                    <line x1="80" y1="540" x2="1080" y2="540" />
                    <line x1="80" y1="640" x2="1080" y2="640" />
                    <line x1="80" y1="740" x2="1080" y2="740" />
                    <line x1="80" y1="840" x2="1080" y2="840" />
                    <line x1="80" y1="940" x2="1080" y2="940" />
                    <line x1="180" y1="40" x2="180" y2="1040" />
                    <line x1="280" y1="40" x2="280" y2="1040" />
                    <line x1="380" y1="40" x2="380" y2="1040" />
                    <line x1="480" y1="40" x2="480" y2="1040" />
                    <line x1="580" y1="40" x2="580" y2="1040" />
                    <line x1="680" y1="40" x2="680" y2="1040" />
                    <line x1="780" y1="40" x2="780" y2="1040" />
                    <line x1="880" y1="40" x2="880" y2="1040" />
                    <line x1="980" y1="40" x2="980" y2="1040" />
                </g>
                <g>{grid}</g>
                <polygon points={square} fill={square_fill} fill-opacity="0.22" stroke={square_fill} stroke-width="3" />
                <g>{eigen_lines}</g>
                <line x1={Self::OX} y1={Self::OY} x2={ix} y2={iy} stroke={BLUE} stroke-width="7" stroke-linecap="round" />
                <circle cx={ix} cy={iy} r="12" fill={BLUE} />
                <text x={ix + 16.0} y={iy - 16.0} font-family="JetBrains Mono" font-size="34" fill={BLUE}>"î"</text>
                <line x1={Self::OX} y1={Self::OY} x2={jx} y2={jy} stroke={ORANGE} stroke-width="7" stroke-linecap="round" />
                <circle cx={jx} cy={jy} r="12" fill={ORANGE} />
                <text x={jx + 16.0} y={jy - 16.0} font-family="JetBrains Mono" font-size="34" fill={ORANGE}>"ĵ"</text>
                <circle cx={Self::OX} cy={Self::OY} r="7" fill={INK} />

                <rect x={PANEL_X} y="0" width={1920.0 - PANEL_X} height="1080" fill={PAPER} />
                <line x1={PANEL_X} y1="0" x2={PANEL_X} y2="1080" stroke="#e4e1da" stroke-width="2" />
                {panel(&frame, 1, "Matrices move space", [
                    "A 2×2 matrix says where the two",
                    "unit arrows land. Every other point",
                    "follows and the grid stays straight.",
                    "Green lines only stretch: eigenvectors.",
                ], facts, "Live: point at the picture to move î.")}
            </g>
        )
    }
}

/// Idea 3: a square wave is a sum of circles turning at odd multiples of one speed. More circles
/// sharpen the corners, but the overshoot at the jump stays near 9%.
#[derive(Debug)]
pub struct FourierScene;

/// Circles drawn at most; unused ones stay in the tree, hidden, so it keeps its shape.
pub const MAX_CIRCLES: usize = 24;

impl FourierScene {
    /// How many circles: a typed value (`circles`), else one more every second up to 12; adjusted
    /// by the viewer's + and − in the live runtime.
    pub fn circles(input: &Input, seconds: f32) -> usize {
        let growing = input
            .value("circles")
            .and_then(|text| text.trim().parse::<i64>().ok())
            .unwrap_or_else(|| (1 + seconds as i64).min(12));
        let adjusted = growing + i64::from(input.count("more")) - i64::from(input.count("fewer"));
        adjusted.clamp(1, MAX_CIRCLES as i64) as usize
    }
}

impl Scene for FourierScene {
    fn name(&self) -> &'static str {
        "Fourier"
    }

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(SCENE_SECONDS)
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        const CX: f32 = 320.0;
        const CY: f32 = 540.0;
        const SCALE: f32 = 150.0;
        const WAVE_X: f32 = 640.0;
        const STEPS: usize = 130;
        const STEP_PX: f32 = 4.0;
        const STEP_PHASE: f32 = 0.05;

        let input = ctx.input();
        let terms = Self::circles(input, frame.seconds());
        // Live, the phase follows the session clock so it keeps turning while the timeline pauses.
        let time = if input.time > 0.0 {
            input.time
        } else {
            frame.seconds()
        };
        let phase = TAU * 0.3 * time;

        let mut center = (CX, CY);
        let circles = (0..MAX_CIRCLES)
            .map(|term| {
                let (k, amplitude) = math::fourier_term(term);
                let r = amplitude * SCALE;
                let next = (center.0 + r * (k * phase).cos(), center.1 - r * (k * phase).sin());
                let visible = u8::from(term < terms);
                let drawn = fframes::svgr!(
                    <g opacity={visible}>
                        <circle cx={center.0} cy={center.1} r={r} fill="none" stroke="#c9c5bc" stroke-width="2" />
                        <line x1={center.0} y1={center.1} x2={next.0} y2={next.1} stroke={INK} stroke-width="3" />
                    </g>
                );
                if term < terms {
                    center = next;
                }
                drawn
            })
            .collect::<Vec<_>>();
        let tip = center;

        let wave = (0..=STEPS)
            .map(|step| {
                let x = WAVE_X + STEP_PX * step as f32;
                let y =
                    CY - SCALE * math::fourier_partial_sum(phase - STEP_PHASE * step as f32, terms);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let target = (0..=STEPS)
            .map(|step| {
                let x = WAVE_X + STEP_PX * step as f32;
                let y = CY - SCALE * math::square_wave(phase - STEP_PHASE * step as f32);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ");

        let peak = math::fourier_peak(terms);
        let error = (math::fourier_partial_sum(PI / 2.0, terms) - 1.0).abs();
        let facts = [
            "f(x) = (4/π)·∑ sin(kx)/k".to_string(),
            "       k = 1, 3, 5, ...".to_string(),
            format!("circles: N = {terms}"),
            format!("error at x = π/2: {}", num(error, 3)),
            format!(
                "peak {}: overshoot {}%",
                num(peak, 3),
                num((peak - 1.0) / 2.0 * 100.0, 1)
            ),
        ];

        let more_hover = if input.is_hovered("more") {
            "#2d2d2d"
        } else {
            INK
        };
        let fewer_hover = if input.is_hovered("fewer") {
            "#2d2d2d"
        } else {
            INK
        };
        let bx = PANEL_X + 40.0;

        fframes::svgr!(
            <g>
                <line x1={WAVE_X} y1={CY} x2="1160" y2={CY} stroke="#d6d3cc" stroke-width="2" />
                <polyline points={target} fill="none" stroke="#d6d3cc" stroke-width="3" stroke-dasharray="6 8" />
                <g>{circles}</g>
                <line x1={tip.0} y1={tip.1} x2={WAVE_X} y2={tip.1} stroke={ORANGE} stroke-width="2" stroke-dasharray="6 6" />
                <circle cx={tip.0} cy={tip.1} r="8" fill={ORANGE} />
                <polyline points={wave} fill="none" stroke={ORANGE} stroke-width="5" stroke-linejoin="round" />

                {panel(&frame, 2, "Any wave is circles", [
                    "A square wave is a sum of sines,",
                    "each a circle turning k times as",
                    "fast. More circles, sharper corners,",
                    "but the jump always overshoots.",
                ], facts, "Live: press + and − to change the circles.")}

                <g data-action="fewer" role="button" tabindex="0" aria-label="Fewer circles">
                    <rect x={bx + 380.0} y="880" width="96" height="64" rx="32" fill={fewer_hover} />
                    <text x={bx + 428.0} y="925" text-anchor="middle" font-family="JetBrains Mono" font-size="40" fill="#ffffff" pointer-events="none">"−"</text>
                </g>
                <g data-action="more" role="button" tabindex="0" aria-label="More circles">
                    <rect x={bx + 492.0} y="880" width="96" height="64" rx="32" fill={more_hover} />
                    <text x={bx + 540.0} y="925" text-anchor="middle" font-family="JetBrains Mono" font-size="40" fill="#ffffff" pointer-events="none">"+"</text>
                </g>
            </g>
        )
    }
}

#[derive(Debug)]
pub struct MathIdeas<'a> {
    pub media: &'a MathIdeasMedia,
}

impl Video for MathIdeas<'_> {
    const FPS: usize = 30;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;
    const BACKGROUND_COLOR: Color = Color::hex(PAPER);

    fn duration(&self) -> Duration<'_> {
        Duration::Auto
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn define_scenes(&self) -> Scenes<'_> {
        let scenes: Vec<&dyn Scene> = vec![&EulerScene, &MatrixScene, &FourierScene];
        Scenes::from(scenes)
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080"
                 width={Self::WIDTH} height={Self::HEIGHT}>
                <rect width="1920" height="1080" fill={PAPER} />
                {ctx.render_scenes(&frame)}
            </svg>
        )
    }
}
