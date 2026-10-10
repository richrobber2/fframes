use std::sync::LazyLock;

use fframes::{FFramesContext, Frame, Svgr, animation::Easing};

static TRANSFORMATION: LazyLock<fframes::animation::KeyFramesAnimation<f64>> =
    LazyLock::new(|| fframes::timeline!(at 0.0 => 3.0, animate 0.0_f64 => 1.0, Easing::EaseInOut));

#[derive(Clone, Copy, Default)]
struct EigenDirection {
    vector: [f64; 2],
    value: f64,
    visible: bool,
}

/// Real eigenvectors of a 2 × 2 matrix, including defective and scalar matrices.
fn eigen(matrix: [f64; 4]) -> (&'static str, [EigenDirection; 2]) {
    let [a, b, c, d] = matrix;
    let mut directions = [EigenDirection::default(); 2];
    if b == 0.0 && c == 0.0 && a == d {
        return ("all", directions);
    }
    let discriminant = (a - d) * (a - d) + 4.0 * b * c;
    if discriminant < 0.0 {
        return ("complex", directions);
    }
    let root = discriminant.sqrt();
    let values = [(a + d + root) / 2.0, (a + d - root) / 2.0];
    let count = if root == 0.0 { 1 } else { 2 };
    for index in 0..count {
        let value = values[index];
        // Choose the longer perpendicular to a row of A - λI, avoiding a zero row.
        let first = [-b, a - value];
        let second = [d - value, -c];
        let mut vector = if first[0].hypot(first[1]) >= second[0].hypot(second[1]) {
            first
        } else {
            second
        };
        let length = vector[0].hypot(vector[1]);
        if length > 0.0 {
            vector[0] /= length;
            vector[1] /= length;
            if vector[0] < 0.0 || (vector[0] == 0.0 && vector[1] < 0.0) {
                vector[0] = -vector[0];
                vector[1] = -vector[1];
            }
            directions[index] = EigenDirection {
                vector,
                value,
                visible: true,
            };
        }
    }
    (if count == 1 { "one" } else { "two" }, directions)
}

fn input_number(input: &fframes::Input, key: &str, default: f64, min: f64, max: f64) -> f64 {
    input
        .value(key)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
        .clamp(min, max)
}

fn display_number(value: f64, places: usize) -> String {
    let rounded = if value.abs() < 0.5 * 10.0_f64.powi(-(places as i32)) {
        0.0
    } else {
        value
    };
    format!("{rounded:.places$}")
}

fn arrow(
    origin: [f64; 2],
    end: [f64; 2],
    color: &'static str,
    label: &'static str,
    label_sign: f64,
    width: f64,
    height: f64,
) -> Svgr<'static> {
    let dx = end[0] - origin[0];
    let dy = end[1] - origin[1];
    let distance = dx.hypot(dy);
    let angle = dy.atan2(dx);
    let head = 12.0_f64.min(distance * 0.4);
    let points = format!(
        "{},{} {},{} {},{}",
        end[0],
        end[1],
        end[0] - head * (angle - 0.45).cos(),
        end[1] - head * (angle - 0.45).sin(),
        end[0] - head * (angle + 0.45).cos(),
        end[1] - head * (angle + 0.45).sin()
    );
    let visible = i32::from(distance >= 2.0);
    fframes::svgr!(
        <g>
            <g opacity={visible}>
                <line x1={origin[0]} y1={origin[1]} x2={end[0]} y2={end[1]}
                    stroke={color} stroke-width="3" />
                <polygon points={points} fill={color} />
                <text x={(end[0] + 14.0).clamp(24.0, width - 65.0)}
                    y={(end[1] + label_sign * 20.0).clamp(73.0, height - 62.0)}
                    fill={color} font-size="15">{label}</text>
            </g>
            <circle cx={origin[0]} cy={origin[1]} r="5" fill="none" stroke={color}
                stroke-width="2" opacity={1 - visible} />
        </g>
    )
}

/// Draw the live matrix explorer entirely as a Rust-generated SVG tree.
/// Bound values are a/b/c/d, progress (0..1), eigen, compact, and animate ("1"/"0").
pub fn render<'a>(frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
    let input = ctx.input();
    let compact = input.value("compact") == Some("1");
    let (width, height) = if compact {
        (500.0_f64, 500.0_f64)
    } else {
        (1000.0, 600.0)
    };
    let target = [
        input_number(input, "a", 1.0, -3.0, 3.0),
        input_number(input, "b", 1.0, -3.0, 3.0),
        input_number(input, "c", 0.0, -3.0, 3.0),
        input_number(input, "d", 1.0, -3.0, 3.0),
    ];
    let animate = input
        .value("animate")
        .map_or(input.value("progress").is_none(), |value| value == "1");
    let progress = if animate {
        frame.animate(&TRANSFORMATION)
    } else {
        input_number(input, "progress", 1.0, 0.0, 1.0)
    };
    let current = [
        1.0 + (target[0] - 1.0) * progress,
        target[1] * progress,
        target[2] * progress,
        1.0 + (target[3] - 1.0) * progress,
    ];
    let determinant = current[0] * current[3] - current[1] * current[2];
    // The native compile-time SVG tree omits browser data attributes.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let (eigen_kind, directions) = eigen(current);
    let show_eigen = input.value("eigen") == Some("1");
    let extent = target
        .iter()
        .map(|value| value.abs())
        .chain([(target[0] + target[1]).abs(), (target[2] + target[3]).abs()])
        .fold(3.3_f64, f64::max)
        * 1.16;
    let scale = (width - 70.0).min(height - 120.0) / (2.0 * extent);
    let origin = [width / 2.0, height / 2.0 + 7.0];
    let project = |point: [f64; 2]| [origin[0] + point[0] * scale, origin[1] - point[1] * scale];
    let transformed = |point: [f64; 2]| {
        project([
            current[0] * point[0] + current[1] * point[1],
            current[2] * point[0] + current[3] * point[1],
        ])
    };

    // Fixed child counts let fframes-live patch the same SVG nodes on every frame.
    let original_grid = (-24..=24)
        .map(|index| {
            let x = project([f64::from(index), 0.0])[0];
            let y = project([0.0, f64::from(index)])[1];
            fframes::svgr!(
                <g stroke="#263128" stroke-width="1">
                    <line x1={x} y1="54" x2={x} y2={height - 48.0} />
                    <line x1="22" y1={y} x2={width - 22.0} y2={y} />
                </g>
            )
        })
        .collect::<Vec<_>>();
    let grid = (-16..=16)
        .map(|index| {
            let value = f64::from(index);
            let vertical_start = transformed([value, -30.0]);
            let vertical_end = transformed([value, 30.0]);
            let horizontal_start = transformed([-30.0, value]);
            let horizontal_end = transformed([30.0, value]);
            fframes::svgr!(
                <g>
                    <line x1={vertical_start[0]} y1={vertical_start[1]}
                        x2={vertical_end[0]} y2={vertical_end[1]}
                        stroke={if index == 0 { "#5d795d" } else { "#46623e" }}
                        stroke-width={if index == 0 { 1.5 } else { 1.0 }} />
                    <line x1={horizontal_start[0]} y1={horizontal_start[1]}
                        x2={horizontal_end[0]} y2={horizontal_end[1]}
                        stroke={if index == 0 { "#756784" } else { "#53475f" }}
                        stroke-width={if index == 0 { 1.5 } else { 1.0 }} />
                </g>
            )
        })
        .collect::<Vec<_>>();
    let ticks = (-24..=24)
        .map(|index| {
            let x = project([f64::from(index), 0.0])[0];
            let y = project([0.0, f64::from(index)])[1];
            let x_visible = index != 0 && x > 35.0 && x < width - 35.0;
            let y_visible = index != 0 && y > 76.0 && y < height - 58.0;
            fframes::svgr!(
                <g fill="#91a095" font-size="13">
                    <text x={x.clamp(35.0, width - 35.0)} y={origin[1] + 20.0} text-anchor="middle"
                        opacity={i32::from(x_visible)}>{index.to_string()}</text>
                    <text x={origin[0] - 11.0} y={(y + 4.0).clamp(76.0, height - 58.0)} text-anchor="end"
                        opacity={i32::from(y_visible)}>{index.to_string()}</text>
                </g>
            )
        })
        .collect::<Vec<_>>();
    let square = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let original_square = square
        .map(project)
        .map(|point| format!("{},{}", point[0], point[1]))
        .join(" ");
    let moved_square = square
        .map(transformed)
        .map(|point| format!("{},{}", point[0], point[1]))
        .join(" ");
    let eigen_lines = directions.into_iter().enumerate().map(|(index, direction)| {
        let length = (width + height) / scale;
        let start = project(direction.vector.map(|value| -value * length));
        let end = project(direction.vector.map(|value| value * length));
        let label = project(direction.vector.map(|value| value * (1.85 + index as f64 * 0.55)));
        fframes::svgr!(
            <g opacity={i32::from(show_eigen && direction.visible)}>
                <line x1={start[0]} y1={start[1]} x2={end[0]} y2={end[1]}
                    stroke="#e8bd86" stroke-width="2" stroke-dasharray="6 6" />
                <text x={(label[0] + 10.0).clamp(30.0, width - 118.0)}
                    y={(label[1] - 12.0).clamp(78.0, height - 65.0)}
                    fill="#e8bd86" font-size="15">{format!("value = {}", display_number(direction.value, 2))}</text>
            </g>
        )
    }).collect::<Vec<_>>();
    let collapsed = determinant.abs() < 1e-9;
    let area = if collapsed {
        if current.iter().all(|value| value.abs() < 1e-9) {
            "Area × 0 · collapsed to a point".to_owned()
        } else {
            "Area × 0 · collapsed to a line".to_owned()
        }
    } else {
        format!(
            "Area × {} · orientation {}",
            display_number(determinant.abs(), 3),
            if determinant < 0.0 {
                "reversed"
            } else {
                "preserved"
            }
        )
    };
    // The native compile-time SVG tree omits browser accessibility attributes.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let description = format!(
        "Transformed grid at {:.0} percent. First basis arrow ({}, {}), second basis arrow ({}, {}). Determinant {}. {}.",
        progress * 100.0,
        display_number(current[0], 2),
        display_number(current[2], 2),
        display_number(current[1], 2),
        display_number(current[3], 2),
        display_number(determinant, 3),
        area
    );
    let basis_first = arrow(
        origin,
        project([current[0], current[2]]),
        "#d9f18b",
        "a, c",
        -1.0,
        width,
        height,
    );
    let basis_second = arrow(
        origin,
        project([current[1], current[3]]),
        "#b6a9ee",
        "b, d",
        1.0,
        width,
        height,
    );
    fframes::svgr!(
        <svg xmlns="http://www.w3.org/2000/svg" width={width} height={height}
            viewBox={if compact { "0 0 500 500" } else { "0 0 1000 600" }}
            role="img" aria-label={description}
            data-det={determinant} data-progress={progress}
            data-matrix={format!("{},{},{},{}", current[0], current[1], current[2], current[3])}
            data-eigen-kind={eigen_kind} font-family="DM Sans" font-weight="400">
            <defs>
                <clipPath id="linear-plane-clip">
                    <rect x="22" y="54" width={width - 44.0} height={height - 102.0} />
                </clipPath>
            </defs>
            <rect width={width} height={height} fill="#18201a" />
            <g clip-path="url(#linear-plane-clip)">
                {original_grid}
                <line x1="22" y1={origin[1]} x2={width - 22.0} y2={origin[1]}
                    stroke="#526054" stroke-width="1.3" />
                <line x1={origin[0]} y1="54" x2={origin[0]} y2={height - 48.0}
                    stroke="#526054" stroke-width="1.3" />
                {grid}
                <polygon points={original_square} fill="none" stroke="#a1ada0"
                    stroke-width="1.7" stroke-dasharray="5 5" />
                <polygon points={moved_square} fill="#d9f18b" fill-opacity="0.13"
                    stroke="#c8d89a" stroke-width="1.7" />
                {ticks}
                {eigen_lines}
                {basis_first}
                {basis_second}
            </g>
            <circle cx={origin[0]} cy={origin[1]} r="3.5" fill="#f0f0e7" />
            <g font-size="13" fill="#a2ada4">
                <text x="28" y="32" letter-spacing="1.1">"THE PLANE / EVERY POINT MOVES"</text>
                <text x={origin[0] - 12.0} y={origin[1] + 20.0} text-anchor="end">"0"</text>
                <text x={width - 27.0} y={origin[1] - 12.0} text-anchor="end">"x"</text>
                <text x={origin[0] + 15.0} y="68">"y"</text>
                <text x="28" y={height - 19.0}>{area}</text>
            </g>
        </svg>
    )
}
