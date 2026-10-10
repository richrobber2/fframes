use std::sync::{Mutex, OnceLock, PoisonError};

use fframes::{
    AudioMap, Color, FFramesContext, FontQuery, Frame, Input, InteractionEvent, Svgr, Transform,
    animation::Easing,
};

use crate::{MotionGraphicsMedia, SPRING_SNAPPY};

const MAX_FONT_SIZE: usize = 500;
const MIN_FONT_SIZE: usize = 60;
const HORIZONTAL_PADDING: usize = 200;
const TARGET_WIDTH: usize = 1920 - HORIZONTAL_PADDING * 2;
const LINE_HEIGHT_RATIO: f32 = 0.9;

/// A single giant word/phrase that fills the entire frame.
/// Supports multiple lines separated by `\n`.
/// Meant to render behind a speaker as a background emphasis card.
///
/// Live (fframes-live) the viewer can type the text (`data-fframes-value="quote"`), click the card
/// to replay its entrance, and the card drifts a little with the pointer.
pub struct QuoteCardVideo<'a> {
    pub media: &'a MotionGraphicsMedia,
    pub text: &'a str,
    resolved_font_sizes: OnceLock<Vec<usize>>,
    /// Font sizes of the last text typed live, measured once per change.
    live_font_sizes: Mutex<Option<(String, Vec<usize>)>>,
    /// `Input::time` when the entrance was last restarted live (new text or a click).
    restarted_at: Mutex<Option<f32>>,
}

impl std::fmt::Debug for QuoteCardVideo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuoteCardVideo")
            .field("text", &self.text)
            .finish()
    }
}

impl<'a> QuoteCardVideo<'a> {
    pub fn new(media: &'a MotionGraphicsMedia, text: &'a str) -> Self {
        Self {
            media,
            text,
            resolved_font_sizes: OnceLock::new(),
            live_font_sizes: Mutex::new(None),
            restarted_at: Mutex::new(None),
        }
    }
}

/// Largest font size (down to [`MIN_FONT_SIZE`]) at which `line` fits into
/// [`TARGET_WIDTH`].  `None` when the font can not be measured yet (fonts are
/// resolved lazily in the editor), so the caller must not cache the result.
fn resolve_font_size<'a>(
    frame: &mut Frame,
    ctx: &FFramesContext<'a, '_>,
    line: &'a str,
) -> Option<usize> {
    let mut size = MAX_FONT_SIZE;
    loop {
        let query = FontQuery {
            family: "Bebas Neue",
            size,
            weight: 400,
            ..Default::default()
        };
        match frame.text_width(ctx, query, line)? {
            w if w <= TARGET_WIDTH => break Some(size),
            _ if size <= MIN_FONT_SIZE => break Some(MIN_FONT_SIZE),
            _ => size -= 10,
        }
    }
}

impl fframes::Video for QuoteCardVideo<'_> {
    const FPS: usize = 60;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;
    const BACKGROUND_COLOR: Color = Color::BLACK;

    fn duration(&self) -> fframes::Duration<'_> {
        fframes::Duration::Seconds(3.0)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn on_event(&self, event: &InteractionEvent, input: &Input) {
        let restart = match event {
            InteractionEvent::Action(action) => action == "replay",
            InteractionEvent::Value { key, .. } => key == "quote",
            _ => false,
        };
        if restart {
            *self
                .restarted_at
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(input.time);
        }
    }

    fn render_frame<'a>(&'a self, mut frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let input = ctx.input();
        let text = input
            .value("quote")
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(self.text);
        let lines: Vec<&str> = text.lines().collect();
        // Measuring text is only cached once the fonts could actually be
        // measured; until then every line falls back to the minimum size.
        let measure = |frame: &mut Frame| {
            lines
                .iter()
                .map(|line| resolve_font_size(frame, ctx, line))
                .collect::<Option<Vec<usize>>>()
        };
        let font_sizes = if text == self.text {
            match self.resolved_font_sizes.get() {
                Some(sizes) => sizes.clone(),
                None => match measure(&mut frame) {
                    Some(sizes) => self.resolved_font_sizes.get_or_init(|| sizes).clone(),
                    None => vec![MIN_FONT_SIZE; lines.len()],
                },
            }
        } else {
            let mut cache = self
                .live_font_sizes
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            match cache.as_ref() {
                Some((cached, sizes)) if cached == text => sizes.clone(),
                _ => match measure(&mut frame) {
                    Some(sizes) => {
                        *cache = Some((text.to_string(), sizes.clone()));
                        sizes
                    }
                    None => vec![MIN_FONT_SIZE; lines.len()],
                },
            }
        };

        // Calculate total block height to center vertically
        let total_height: f32 = font_sizes
            .iter()
            .map(|s| *s as f32 * LINE_HEIGHT_RATIO)
            .sum();
        let start_y = 540.0 - total_height / 2.0;

        // Live, a click or new text plays the entrance again from that moment.
        let restarted = (*self
            .restarted_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner))
        .map(|at| input.time - at)
        .filter(|&since| since < 2.2);
        let sample = |animation: &fframes::animation::KeyFramesAnimation<f32>| match restarted {
            Some(since) => frame.animate_at(animation, since),
            None => frame.animate(animation),
        };

        // Scale punch in (start big, spring down to 1.0)
        let scale = sample(&fframes::timeline!(
            at 0.0 => 0.8, animate 1.6_f32 => 1.0, SPRING_SNAPPY,
        ));

        // Slide up from below
        let slide_y = sample(&fframes::timeline!(
            at 0.0 => 0.6, animate 80.0_f32 => 0.0, SPRING_SNAPPY,
        ));

        // Fade in
        let opacity = sample(&fframes::timeline!(
            at 0.0 => 0.3, animate 0.0_f32 => 1.0, Easing::EaseOut,
        ));

        // Fade out
        let fade_out = sample(&fframes::timeline!(
            at 2.2 => 2.8, animate 1.0_f32 => 0.0, Easing::EaseIn,
        ));

        // A little depth live: the card leans away from the pointer. Nothing in file renders.
        let (drift_x, drift_y) = input.pointer.map_or((0.0, 0.0), |(x, y)| {
            (
                f64::from((960.0 - x) / 960.0 * 28.0),
                f64::from((540.0 - y) / 540.0 * 18.0),
            )
        });

        let mut y_offset = start_y;
        let text_elements: Vec<_> = lines
            .iter()
            .zip(font_sizes.iter())
            .map(|(line, &size)| {
                let line_y = y_offset + size as f32 * LINE_HEIGHT_RATIO;
                y_offset = line_y;
                let y = line_y;

                fframes::svgr!(
                    <text
                        x="960"
                        y={y}
                        font-family="Bebas Neue"
                        font-size={size}
                        fill="white"
                        text-anchor="middle"
                        letter-spacing="20"
                    >
                        {*line}
                    </text>
                )
            })
            .collect();

        fframes::svgr!(
            <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 1920 1080"
                width={Self::WIDTH}
                height={Self::HEIGHT}
            >
                <rect width={Self::WIDTH} height={Self::HEIGHT} fill="black" />

                <g
                    data-action="replay"
                    role="button"
                    tabindex="0"
                    aria-label="Replay the entrance"
                    opacity={opacity * fade_out}
                    transform={Transform {
                        translate_x: 960.0 * (1.0 - f64::from(scale)) + drift_x,
                        translate_y: 540.0 * (1.0 - f64::from(scale)) + f64::from(slide_y) + drift_y,
                        scale: fframes::Scale { x: f64::from(scale), y: f64::from(scale) },
                        ..Default::default()
                    }}
                >
                    {text_elements}
                </g>
            </svg>
        )
    }
}
