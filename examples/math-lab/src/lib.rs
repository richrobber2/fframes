mod chaos;
mod fourier;
mod linear;

use fframes::{AudioMap, Color, Duration, FFramesContext, Frame, Svgr, Video, include_media_dir};

include_media_dir!(pub struct MathLabMedia, "examples/math-lab/media");

/// The three mathematical experiments supported by the shared live bridge.
#[derive(Clone, Copy, Debug)]
pub enum Lab {
    Fourier,
    Linear,
    Chaos,
}

impl Lab {
    /// Resolve the browser's experiment identifier.
    pub fn from_name(name: &str) -> Self {
        match name {
            "linear" => Self::Linear,
            "chaos" => Self::Chaos,
            _ => Self::Fourier,
        }
    }
}

/// A normal fframes video, also rendered interactively by fframes-live.
pub struct MathLab {
    lab: Lab,
    chaos: Option<chaos::Chaos>,
}

impl MathLab {
    /// Prepare static geometry once, before the first live frame.
    pub fn new(lab: Lab) -> Self {
        Self {
            lab,
            chaos: matches!(lab, Lab::Chaos).then(chaos::Chaos::new),
        }
    }
}

impl Video for MathLab {
    const FPS: usize = 30;
    const WIDTH: usize = 1000;
    const HEIGHT: usize = 600;
    const BACKGROUND_COLOR: Color = Color::hex("#111813");

    fn duration(&self) -> Duration<'_> {
        // 32 seconds gives a whole number of Fourier cycles for every 0.25x speed.
        Duration::Seconds(match self.lab {
            Lab::Fourier => 32.0,
            Lab::Linear => 4.0,
            Lab::Chaos => 1.0,
        })
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        match self.lab {
            Lab::Fourier => fourier::render(frame, ctx),
            Lab::Linear => linear::render(frame, ctx),
            Lab::Chaos => self
                .chaos
                .as_ref()
                .map_or_else(Svgr::empty, |scene| scene.render(frame, ctx)),
        }
    }
}
