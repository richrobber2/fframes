use fframes::{
    AudioMap, Color, FFramesContext, Frame, Scene, Transform, Video, animation::Easing,
    include_media_dir,
};

include_media_dir!(pub struct HelloWorldMedia, "examples/hello-world/media");

#[derive(Debug)]
pub struct HelloWorldVideo<'a> {
    pub slug: &'a str,
    pub media: &'a HelloWorldMedia,
}

impl Video for HelloWorldVideo<'_> {
    const FPS: usize = 30;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;

    fn duration(&self) -> fframes::Duration<'_> {
        fframes::Duration::Auto
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn define_scenes(&self) -> fframes::Scenes<'_> {
        fframes::Scenes::from(vec![self as &dyn Scene, &LoadingBar])
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> fframes::Svgr<'a> {
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080" width="1920" height="1080">
                {ctx.render_scenes(&frame)}
            </svg>
        )
    }
}

impl Scene for HelloWorldVideo<'_> {
    fn name(&self) -> &'static str {
        "Hello World"
    }
    fn duration(&self) -> fframes::Duration<'_> {
        fframes::Duration::Seconds(30.)
    }

    fn render_frame(&self, frame: Frame, ctx: &FFramesContext) -> fframes::Svgr<'_> {
        fframes::svgr!(
            <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 1920 1080"
                width={ctx.current_video_size.width}
                height={ctx.current_video_size.height}
            >
                <rect
                    width={ctx.current_video_size.width}
                    height={ctx.current_video_size.height}
                    fill={frame.animate(&fframes::timeline!(
                        at 0., animate Color::hex("#fff") => Color::hex("#f8fafc"), Easing::Linear,
                        at 5., animate Color::hex("#f8fafc") => Color::hex("#fff7ed"), Easing::Linear,
                        at 10., animate Color::hex("#fff7ed") => Color::hex("#fef2f2"), Easing::Linear,
                        at 15., animate Color::hex("#fef2f2") => Color::hex("#f7fee7"), Easing::Linear,
                        at 20., animate Color::hex("#f7fee7") => Color::hex("#ecfdf5"), Easing::Linear,
                        at 25. => 30., animate Color::hex("#ecfdf5") => Color::hex("#faf5ff"), Easing::Linear
                    ))}
                />

                // renders under the text
                <rect
                    x="400"
                    y="400"
                    width="200"
                    height="200"
                    fill="blue"
                    transform={frame.animate(fframes::timeline!(
                        at 0., animate Transform::translate(0, 0) => Transform::translate(200, 480), Easing::Linear,
                        at 2. => 6.0, animate Transform::translate(200, 480) => Transform::translate(750, -400), Easing::Linear,
                        at 6.0 => 10.0, animate Transform::translate(750, -400) => Transform::translate(1310, 480), Easing::Linear,
                    ))}
                />

                <text
                    x="99"
                    y="300"
                    font-family="DM Sans"
                    font-size="150"
                    fill="#000"
                >
                    "Hello " {self.slug}
                </text>

                <text
                    x="100"
                    y="440"
                    font-family="JetBrains Mono"
                    font-size="74"
                    font-weight="500"
                    fill="#3b5563"
                >
                    {format!("This frame index: {}, second: {:.2}", frame.index, frame.seconds())}
                </text>
            </svg>
        )
    }
}

#[derive(Debug)]
struct LoadingBar;

impl Scene for LoadingBar {
    fn name(&self) -> &'static str {
        "Loading Bar"
    }
    fn duration(&self) -> fframes::Duration<'_> {
        fframes::Duration::Seconds(8.)
    }

    fn render_frame(&self, frame: Frame, _ctx: &FFramesContext) -> fframes::Svgr<'_> {
        let progress = ((frame.seconds() - 0.5) / 6.0).clamp(0.0, 1.0);
        let progress = progress * progress * (3.0 - 2.0 * progress);
        let percent = (progress * 100.0).round() as u32;
        fframes::svgr!(
            <rect width="1920" height="1080" fill="#10151e" />
            <text x="240" y="365" font-family="DM Sans" font-size="92" font-weight="700" fill="#f8fafc">
                {if percent == 100 { "Ready to go" } else { "Loading" }}
            </text>
            <text x="240" y="440" font-family="DM Sans" font-size="36" fill="#a2afc0"> "A little progress, frame by frame." </text>
            <rect x="240" y="515" width="1440" height="64" rx="32" fill="#293444" />
            <rect x="240" y="515" width={1440.0 * progress} height="64" rx="32" fill="#ff6922" />
            <text x="240" y="690" font-family="JetBrains Mono" font-size="72" fill="#f8fafc"> {format!("{percent}%")} </text>
            <text x="1680" y="690" text-anchor="end" font-family="DM Sans" font-size="36" fill="#a2afc0">
                {if percent == 100 { "Complete" } else { "Preparing" }}
            </text>
        )
    }
}
