use std::fmt::Write;

use fframes::{FFramesContext, Frame, Input, Svgr};

const STEPS: usize = 80;
const COLUMNS: usize = 601;
const RETAINED: usize = 48;
const BURN_IN: usize = 1200;

#[derive(Clone, Copy, Debug)]
struct Plot {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Plot {
    fn x_at(self, fraction: f64) -> f64 {
        self.x + fraction * self.width
    }

    fn y_at(self, value: f64) -> f64 {
        self.y + (1.0 - value) * self.height
    }
}

#[derive(Clone, Copy, Debug)]
struct Layout {
    width: u32,
    height: u32,
    view_box: &'static str,
    map: Plot,
    a: Plot,
    b: Plot,
    divider_y: f64,
}

const DESKTOP: Layout = Layout {
    width: 1000,
    height: 600,
    view_box: "0 0 1000 600",
    map: Plot {
        x: 58.0,
        y: 64.0,
        width: 906.0,
        height: 244.0,
    },
    a: Plot {
        x: 58.0,
        y: 408.0,
        width: 406.0,
        height: 145.0,
    },
    b: Plot {
        x: 558.0,
        y: 408.0,
        width: 406.0,
        height: 145.0,
    },
    divider_y: 356.0,
};

const COMPACT: Layout = Layout {
    width: 500,
    height: 900,
    view_box: "0 0 500 900",
    map: Plot {
        x: 52.0,
        y: 66.0,
        width: 414.0,
        height: 258.0,
    },
    a: Plot {
        x: 52.0,
        y: 427.0,
        width: 414.0,
        height: 147.0,
    },
    b: Plot {
        x: 52.0,
        y: 677.0,
        width: 414.0,
        height: 147.0,
    },
    divider_y: 371.0,
};

/// Interactive bifurcation and trajectory plots for the logistic map.
#[derive(Debug)]
pub struct Chaos {
    cloud_desktop: String,
    cloud_compact: String,
}

impl Default for Chaos {
    fn default() -> Self {
        Self::new()
    }
}

impl Chaos {
    /// Computes the long-run cloud once, with 1,200 discarded iterations per column.
    pub fn new() -> Self {
        let mut cloud_desktop = String::with_capacity(COLUMNS * RETAINED * 23);
        let mut cloud_compact = String::with_capacity(COLUMNS * RETAINED * 23);
        for column in 0..COLUMNS {
            let fraction = column as f64 / (COLUMNS - 1) as f64;
            let rate = 2.5 + 1.5 * fraction;
            let mut value = 0.314_159_265_358_979_3;
            for _ in 0..BURN_IN {
                value = next(rate, value);
            }
            for _ in 0..RETAINED {
                value = next(rate, value);
                // Short, round-capped segments make dots without thousands of DOM nodes.
                append_dot(&mut cloud_desktop, DESKTOP.map, fraction, value);
                append_dot(&mut cloud_compact, COMPACT.map, fraction, value);
            }
        }
        Self {
            cloud_desktop,
            cloud_compact,
        }
    }

    /// Reads live controls and returns a complete SVG; file renders use meaningful defaults.
    pub fn render<'a>(&'a self, _frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let input = ctx.input();
        let rate = number(input, "rate", 3.9, 2.5, 4.0);
        let initial = number(input, "initial", 0.4, 0.05, 0.95);
        let compact = input.value("compact") == Some("1");
        let layout = if compact { COMPACT } else { DESKTOP };
        let cloud = if compact {
            self.cloud_compact.as_str()
        } else {
            self.cloud_desktop.as_str()
        };
        let a = trajectory(rate, initial);
        let b = trajectory(rate, initial + 0.0001);
        let final_gap = (a[STEPS] - b[STEPS]).abs();
        let mut max_gap = 0.0;
        let mut max_step = 0;
        for step in 0..=STEPS {
            let gap = (a[step] - b[step]).abs();
            if gap > max_gap {
                max_gap = gap;
                max_step = step;
            }
        }

        let marker_x = layout.map.x_at((rate - 2.5) / 1.5);
        let marker_arrow = format!("M{:.2} {:.2}h10l-5 7z", marker_x - 5.0, layout.map.y - 14.0);
        let map_grid = grid(layout.map, &[2.5, 3.0, 3.5, 4.0], 2.5, 4.0, true);
        let a_chart = trace(layout.a, &a, "#d9f18b", "02 / A", initial);
        let b_chart = trace(layout.b, &b, "#b6a9ee", "03 / B", initial + 0.0001);
        let chart_divider = if compact {
            fframes::svgr!(<line x1="26" y1="621" x2="474" y2="621" stroke="#2c382e" />)
        } else {
            fframes::svgr!(<line x1="500" y1="376" x2="500" y2="580" stroke="#2c382e" />)
        };
        // The native compile-time SVG tree omits browser accessibility attributes.
        #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
        let description = format!(
            "Logistic map at r = {rate:.3}. The upper diagram shows long-run values across rates 2.5 to 4. Trajectories A and B start at {initial:.4} and {:.4}. After 80 steps the gap is {final_gap:.6}; the largest gap is {max_gap:.6} at step {max_step}.",
            initial + 0.0001
        );

        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox={layout.view_box}
                width={layout.width} height={layout.height} font-family="DM Sans"
                role="img" aria-label={description}
                data-rate={rate} data-initial={initial}
                data-final-gap={final_gap} data-max-gap={max_gap} data-max-step={max_step}
                data-map-x={layout.map.x} data-map-width={layout.map.width}
                data-map-y={layout.map.y} data-map-height={layout.map.height}>
                <rect width={layout.width} height={layout.height} fill="#19221b" />
                <text x="26" y="32" fill="#9ba99e" font-size="13" font-weight="500"
                    letter-spacing="1.1">"01 / THE LONG-RUN MAP"</text>
                <text x={layout.width - 34} y="32" text-anchor="end" fill="#b6a9ee"
                    font-size="14" font-weight="500">{format!("r = {rate:.3}")}</text>
                {map_grid}
                <path d={cloud} fill="none" stroke="#a7b579" stroke-width="1.05"
                    stroke-linecap="round" opacity="0.68" />
                <line x1={marker_x} y1={layout.map.y - 4.0} x2={marker_x}
                    y2={layout.map.y + layout.map.height} stroke="#b6a9ee" stroke-width="1.5" />
                <path d={marker_arrow} fill="#b6a9ee" />
                <text x={layout.width - 16} y={layout.map.y + layout.map.height + 27.0}
                    text-anchor="end" fill="#9ba99e" font-size="13">"r"</text>
                <line x1="26" y1={layout.divider_y} x2={layout.width - 26}
                    y2={layout.divider_y} stroke="#2c382e" />
                {a_chart}
                {chart_divider}
                {b_chart}
            </svg>
        )
    }
}

fn number(input: &Input, name: &str, fallback: f64, min: f64, max: f64) -> f64 {
    input
        .value(name)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn next(rate: f64, value: f64) -> f64 {
    rate * value * (1.0 - value)
}

fn trajectory(rate: f64, initial: f64) -> [f64; STEPS + 1] {
    let mut values = [0.0; STEPS + 1];
    values[0] = initial;
    for step in 1..=STEPS {
        values[step] = next(rate, values[step - 1]);
    }
    values
}

fn append_dot(path: &mut String, plot: Plot, fraction: f64, value: f64) {
    let _ = write!(
        path,
        "M{:.2} {:.2}h0.05",
        plot.x_at(fraction),
        plot.y_at(value)
    );
}

fn grid(plot: Plot, ticks: &[f64], min: f64, max: f64, decimals: bool) -> Vec<Svgr<'static>> {
    let mut elements = Vec::with_capacity(3 + ticks.len());
    for value in [0.0, 0.5, 1.0] {
        let y = plot.y_at(value);
        elements.push(fframes::svgr!(
            <g>
                <line x1={plot.x} y1={y} x2={plot.x + plot.width} y2={y} stroke="#2c382e" />
                <text x={plot.x - 12.0} y={y + 4.0} text-anchor="end" fill="#9ba99e"
                    font-size="12">{value.to_string()}</text>
            </g>
        ));
    }
    for &tick in ticks {
        let x = plot.x_at((tick - min) / (max - min));
        let label = if decimals {
            format!("{tick:.1}")
        } else {
            format!("{tick:.0}")
        };
        elements.push(fframes::svgr!(
            <g>
                <line x1={x} y1={plot.y} x2={x} y2={plot.y + plot.height} stroke="#2c382e" />
                <text x={x} y={plot.y + plot.height + 23.0} text-anchor="middle"
                    fill="#9ba99e" font-size="12">{label}</text>
            </g>
        ));
    }
    elements
}

fn trace(
    plot: Plot,
    values: &[f64; STEPS + 1],
    color: &'static str,
    label: &'static str,
    initial: f64,
) -> Svgr<'static> {
    let mut path = String::with_capacity((STEPS + 1) * 18);
    for (step, &value) in values.iter().enumerate() {
        let command = if step == 0 { 'M' } else { 'L' };
        let _ = write!(
            path,
            "{command}{:.2} {:.2}",
            plot.x_at(step as f64 / STEPS as f64),
            plot.y_at(value)
        );
    }
    let chart_grid = grid(plot, &[0.0, 20.0, 40.0, 60.0, 80.0], 0.0, 80.0, false);
    fframes::svgr!(
        <g>
            <text x={plot.x - 26.0} y={plot.y - 24.0} fill={color} font-size="14"
                font-weight="500">{format!("{label} · x(0) = {initial:.4}")}</text>
            {chart_grid}
            <path d={path} fill="none" stroke={color} stroke-width="1.65"
                stroke-linejoin="round" stroke-linecap="round" />
            <circle cx={plot.x + plot.width} cy={plot.y_at(values[STEPS])} r="3" fill={color} />
            <text x={plot.x + plot.width / 2.0} y={plot.y + plot.height + 43.0}
                text-anchor="middle" fill="#9ba99e" font-size="12">"Iteration"</text>
        </g>
    )
}
