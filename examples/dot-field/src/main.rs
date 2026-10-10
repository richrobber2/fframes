use dot_field_example::{DotField, DotFieldMedia};
use fframes::{EncoderOptions, RenderOptions, StaticMediaProvider, cli};
use std::process::ExitCode;

fn main() -> ExitCode {
    let media = DotFieldMedia::prepare().unwrap();
    let video = DotField::new(&media);

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
