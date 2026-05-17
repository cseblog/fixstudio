mod app;
mod components;
mod loader;
mod dictionary;
mod export;
mod fill_quality;
mod model;
mod parser;
mod recents;
mod sample;
mod session_health;
mod session_summary;
mod style;
mod tab;
mod types;
mod validator;

use dioxus_desktop::{launch::launch, Config, WindowBuilder};

fn load_icon() -> Option<tao::window::Icon> {
    let bytes = include_bytes!("../assets/icon.ico");
    let dir = ico::IconDir::read(std::io::Cursor::new(bytes)).ok()?;
    let entry = dir.entries().first()?;
    let image = entry.decode().ok()?;
    tao::window::Icon::from_rgba(image.rgba_data().to_vec(), image.width(), image.height()).ok()
}

fn prewarm() {
    // 1. Spin up Rayon thread pool now so first parse doesn't pay thread-spawn cost.
    rayon::ThreadPoolBuilder::new()
        .build_global()
        .ok();

    // 2. Touch a tiny dummy parse so mimalloc TLS arenas and CPU i-cache are warm.
    let dummy = b"8=FIX.4.2\x019=5\x0135=D\x0110=000\x01";
    let _ = parser::parse_all_simd_bytes(dummy);
}

fn main() {
    prewarm();

    let config = Config::default()
        .with_custom_head(
            r#"<script src="https://cdn.jsdelivr.net/npm/echarts@5.5.1/dist/echarts.min.js"></script>"#
                .to_string(),
        )
        .with_window(
            WindowBuilder::new()
                .with_title("AI FIX Parser")
                .with_maximized(true)
                .with_inner_size(dioxus_desktop::LogicalSize::new(1280.0, 900.0))
                .with_window_icon(load_icon()),
        );

    launch(app::app, Vec::new(), vec![Box::new(config)]);
}
