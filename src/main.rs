mod app;
mod components;
mod dictionary;
mod model;
mod parser;
mod sample;
mod style;

use dioxus_desktop::{launch::launch, Config, WindowBuilder};

fn load_icon() -> Option<tao::window::Icon> {
    let bytes = include_bytes!("../assets/icon.ico");
    let dir = ico::IconDir::read(std::io::Cursor::new(bytes)).ok()?;
    let entry = dir.entries().first()?;
    let image = entry.decode().ok()?;
    tao::window::Icon::from_rgba(image.rgba_data().to_vec(), image.width(), image.height()).ok()
}

fn main() {
    let config = Config::default().with_window(
        WindowBuilder::new()
            .with_title("AI FIX Parser")
            .with_maximized(true)
            .with_inner_size(dioxus_desktop::LogicalSize::new(1280.0, 900.0))
            .with_window_icon(load_icon()),
    );

    launch(app::app, Vec::new(), vec![Box::new(config)]);
}
