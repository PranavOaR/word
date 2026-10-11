mod app;
mod document;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 820.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "WordMock",
        options,
        Box::new(|_creation_context| Ok(Box::new(app::WordMockApp::default()))),
    )
}
