use anyhow::Result;
use eframe::egui;
use tagger::App;

fn main() -> Result<()> {
    let app = App::init("test.db")?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("tagboat");
        });
    })?;
    Ok(())
}
