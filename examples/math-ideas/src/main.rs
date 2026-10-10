use fframes::{EncoderOptions, RenderOptions, StaticMediaProvider, cli};
use math_ideas_example::{MathIdeas, MathIdeasMedia};
use std::process::ExitCode;

fn main() -> ExitCode {
    let media = MathIdeasMedia::prepare().unwrap();
    let video = MathIdeas { media: &media };

    cli::new(
        &video,
        RenderOptions {
            media: Some(&media),
            video_encoder_options: EncoderOptions {
                preferred_encoder: Some("libx264"),
                codec_params: Some(&[("crf", "20"), ("preset", "veryfast")]),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .run()
}
