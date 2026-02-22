mod app;
mod components;
mod dictionary;
mod model;
mod parser;
mod sample;
mod style;

use dioxus_desktop::{launch::launch, Config, WindowBuilder};

fn main() {
    
    let config = Config::default().with_window(
        WindowBuilder::new()
            .with_title("AI FIX Engine")
            .with_inner_size(dioxus_desktop::LogicalSize::new(1280.0, 900.0)),
    );

    launch(app::app, Vec::new(), vec![Box::new(config)]);
}
