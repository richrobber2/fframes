use fframes::{
    RenderOptions, StaticMediaProvider,
    cli::{self, clap},
};
use math_lab::{Lab, MathLab, MathLabMedia};

#[derive(Debug, clap::Args)]
struct Options {
    /// Which live experiment to inspect or render.
    #[arg(long, default_value = "fourier", value_parser = ["fourier", "linear", "chaos"], global = true)]
    lab: String,
}

fn main() -> std::process::ExitCode {
    let args = cli::parse::<Options>();
    let media = match MathLabMedia::prepare() {
        Ok(media) => media,
        Err(error) => {
            eprintln!("Could not load math-lab media: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let video = MathLab::new(Lab::from_name(&args.app.lab));
    cli::new(
        &video,
        RenderOptions {
            media: Some(&media),
            default_font: "DM Sans",
            ..Default::default()
        },
    )
    .args(args)
    .run()
}
