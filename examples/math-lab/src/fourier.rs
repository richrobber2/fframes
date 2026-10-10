use std::{
    f64::consts::{PI, TAU},
    fmt::Write,
};

use fframes::{FFramesContext, Frame, Svgr};

fn number(ctx: &FFramesContext<'_, '_>, key: &str, default: f64, min: f64, max: f64) -> f64 {
    ctx.input()
        .value(key)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
        .clamp(min, max)
}

fn ideal(wave: &str, phase: f64) -> f64 {
    match wave {
        "triangle" => 2.0 / PI * phase.sin().asin(),
        "sawtooth" => {
            let wrapped = (phase + PI).rem_euclid(TAU) - PI;
            if (wrapped.abs() - PI).abs() < 1e-10 {
                0.0
            } else {
                wrapped / PI
            }
        }
        _ => {
            if phase.sin().abs() < 1e-10 {
                0.0
            } else {
                phase.sin().signum()
            }
        }
    }
}

pub fn render<'a>(frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
    let input = ctx.input();
    let compact = input.value("compact") == Some("1");
    let wave = input.value("wave").unwrap_or("square");
    let count = number(ctx, "terms", 5.0, 1.0, 30.0).round() as usize;
    let speed = number(ctx, "speed", 1.0, 0.25, 2.0);
    let phase = if input.value("manual") == Some("1") {
        number(ctx, "phase", 28.0, 0.0, 360.0).to_radians()
    } else {
        (f64::from(frame.seconds()) * TAU / 8.0 * speed).rem_euclid(TAU)
    };
    let show_target = input.value("target") != Some("0");
    let terms: Vec<(f64, f64)> = (0..count)
        .map(|i| {
            let n = if wave == "sawtooth" { i + 1 } else { 2 * i + 1 } as f64;
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            let amplitude = match wave {
                "triangle" => 8.0 * sign / (PI * PI * n * n),
                "sawtooth" => 2.0 * sign / (PI * n),
                _ => 4.0 / (PI * n),
            };
            (n, amplitude)
        })
        .collect();
    let extent = terms.iter().map(|(_, a)| a.abs()).sum::<f64>();
    let (width, height) = if compact {
        (500.0, 760.0)
    } else {
        (1000.0, 600.0)
    };
    let (cx, cy, gx, gy, gw) = if compact {
        (250.0, 208.0, 54.0, 564.0, 416.0)
    } else {
        (245.0, 296.0, 554.0, 296.0, 412.0)
    };
    let scale = if compact {
        (155.0 / extent).min(100.0)
    } else {
        (211.0 / extent).min(115.0)
    };
    let mut x = 0.0;
    let mut y = 0.0;
    let circles: Vec<_> = terms.iter().map(|(n, amplitude)| {
        let px = cx + x * scale;
        let py = cy - y * scale;
        x += amplitude * (n * phase).cos();
        y += amplitude * (n * phase).sin();
        let ex = cx + x * scale;
        let ey = cy - y * scale;
        fframes::svgr!(
            <g>
                <circle cx={px} cy={py} r={amplitude.abs()*scale} fill="none" stroke="#9f8ece" stroke-width="1.5" />
                <line x1={px} y1={py} x2={ex} y2={ey} stroke="#c4b9ed" stroke-width="1.8" />
                <circle cx={ex} cy={ey} r="2.7" fill="#c4b9ed" />
            </g>
        )
    }).collect();
    let mut curve = String::with_capacity(14000);
    let mut target = String::with_capacity(14000);
    let mut previous = 0.0;
    let mut previous_is_jump = false;
    for i in 0..=600 {
        let offset = f64::from(i) / 600.0;
        let t = phase - offset * TAU;
        let value: f64 = terms.iter().map(|(n, a)| a * (n * t).sin()).sum();
        let goal = ideal(wave, t);
        let is_jump = match wave {
            "triangle" => false,
            "sawtooth" => {
                let wrapped = (t + PI).rem_euclid(TAU) - PI;
                (wrapped.abs() - PI).abs() < 1e-10
            }
            _ => t.sin().abs() < 1e-10,
        };
        let path_command = if i == 0 { 'M' } else { 'L' };
        let target_command =
            if i == 0 || is_jump || previous_is_jump || (goal - previous).abs() > 1.0 {
                'M'
            } else {
                'L'
            };
        let _ = write!(
            curve,
            "{path_command}{:.2},{:.2} ",
            gx + offset * gw,
            gy - value * scale
        );
        let _ = write!(
            target,
            "{target_command}{:.2},{:.2} ",
            gx + offset * gw,
            gy - goal * scale
        );
        previous = goal;
        previous_is_jump = is_jump;
    }
    let axes: Vec<_> = [-1.0, 0.0, 1.0].into_iter().map(|level| {
        let yy = gy - level * scale;
        fframes::svgr!(
            <g>
                <line x1={gx} y1={yy} x2={gx+gw} y2={yy} stroke="#303c32" stroke-width="1" />
                <text x={gx-12.0} y={yy+5.0} text-anchor="end" fill="#a2ada4" font-size="15">{format!("{level:.0}")}</text>
            </g>
        )
    }).collect();
    let endpoint_x = cx + x * scale;
    let endpoint_y = cy - y * scale;
    fframes::svgr!(
        <svg xmlns="http://www.w3.org/2000/svg" viewBox={format!("0 0 {width} {height}")} width={width} height={height}
             role="img" aria-label="Fourier series: rotating circles and the wave traced by their combined height."
             font-family="DM Sans" font-weight="500" data-phase={phase.to_degrees()} data-value={y}>
            <defs>
                <pattern id="fourier-dots" x="0" y="0" width="28" height="28" patternUnits="userSpaceOnUse">
                    <circle cx="14" cy="14" r="0.8" fill="#2b382e" />
                </pattern>
            </defs>
            <rect width={width} height={height} fill="#111813" />
            <rect width={width} height={height} fill="url(#fourier-dots)" />
            <text x="28" y="32" fill="#a2ada4" font-size="15" letter-spacing="1">"01 / ROTATION"</text>
            <text x={if compact { 28.0 } else { gx }} y={if compact { 404.0 } else { 32.0 }} fill="#a2ada4" font-size="15" letter-spacing="1">"02 / FOURIER SUM"</text>
            <line x1="25" y1={cy} x2={if compact { 475.0 } else { 495.0 }} y2={cy} stroke="#303c32" />
            <line x1={cx} y1="48" x2={cx} y2={if compact { 372.0 } else { 535.0 }} stroke="#303c32" />
            <g>{axes}</g>
            <g>{circles}</g>
            <circle cx={cx} cy={cy} r="3.5" fill="#c4b9ed" />
            <line x1={endpoint_x} y1={endpoint_y} x2={gx} y2={gy-y*scale} stroke="#7e9855" stroke-dasharray="4 6" opacity={i32::from(!compact)} />
            <path d={target} fill="none" stroke="#89978a" stroke-width="1.5" stroke-dasharray="5 7" opacity={i32::from(show_target)} />
            <path id="fourier-wave" d={curve} fill="none" stroke="#d9f18b" stroke-width="2.8" stroke-linejoin="round" />
            <circle cx={endpoint_x} cy={endpoint_y} r="10" fill="#d9f18b" fill-opacity="0.12" />
            <circle cx={endpoint_x} cy={endpoint_y} r="5" fill="#d9f18b" />
            <circle cx={gx} cy={gy-y*scale} r="5" fill="#d9f18b" />
            <text x={gx} y={height-26.0} fill="#a2ada4" font-size="14">"NOW"</text>
            <text x={gx+gw} y={height-26.0} text-anchor="end" fill="#a2ada4" font-size="14">"ONE CYCLE AGO"</text>
        </svg>
    )
}
