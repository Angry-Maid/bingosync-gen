use std::{env, fs::File, io::BufWriter};

use egui::{FontFamily, FontId, TextStyle, Vec2};
use rfd::FileDialog;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BingoSyncGen {
    #[serde(skip)]
    board: [Box<String>; 25],

    #[serde(skip)]
    generated: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Card {
    name: String,
}

impl Default for BingoSyncGen {
    fn default() -> Self {
        Self {
            board: core::array::from_fn(|idx| Box::new(String::from(""))),
            generated: String::from(""),
        }
    }
}

#[inline]
fn heading2() -> TextStyle {
    TextStyle::Name("Heading2".into())
}

#[inline]
fn heading3() -> TextStyle {
    TextStyle::Name("ContextHeading".into())
}

fn configure_text_styles(ctx: &egui::Context) {
    use FontFamily::Proportional;
    use TextStyle::*;

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (Heading, FontId::new(24.0, Proportional)),
        (heading2(), FontId::new(22.0, Proportional)),
        (heading3(), FontId::new(20.0, Proportional)),
        (Body, FontId::new(18.0, Proportional)),
        (Monospace, FontId::new(20.0, Proportional)),
        (Button, FontId::new(18.0, Proportional)),
        (Small, FontId::new(18.0, Proportional)),
    ]
    .into();
    ctx.set_style(style);
}

impl BingoSyncGen {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_text_styles(&cc.egui_ctx);
        Default::default()
    }
}

impl eframe::App for BingoSyncGen {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.generated = serde_json::to_string(&self.board.clone().map(|item| Card {
            name: *item.to_owned(),
        }))
        .unwrap();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        let save_path = FileDialog::new()
                            .add_filter("output", &["txt", "json"])
                            .set_directory(env::current_dir().unwrap())
                            .save_file();
                        if let Some(path) = save_path {
                            let file = File::create(path).unwrap();
                            let mut writer = BufWriter::new(file);
                            serde_json::to_writer(
                                &mut writer,
                                &self.board.clone().map(|item| Card {
                                    name: *item.to_owned(),
                                }),
                            )
                            .unwrap();
                        }
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("bingo_grid")
                .num_columns(5)
                .spacing([16.0, 16.0])
                .show(ui, |ui| {
                    for c in 0..5 {
                        for r in 0..5 {
                            ui.add_sized(
                                Vec2::new(128.0, 128.0),
                                egui::TextEdit::multiline(&mut *self.board[c * 5 + r])
                                    .font(TextStyle::Monospace),
                            );
                        }
                        ui.end_row();
                    }
                });

            ui.separator();

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Generated");
                    if ui.button("Save").clicked() {
                        let save_path = FileDialog::new()
                            .add_filter("output", &["txt", "json"])
                            .set_directory(env::current_dir().unwrap())
                            .save_file();
                        if let Some(path) = save_path {
                            let file = File::create(path).unwrap();
                            let mut writer = BufWriter::new(file);
                            serde_json::to_writer(
                                &mut writer,
                                &self.board.clone().map(|item| Card {
                                    name: *item.to_owned(),
                                }),
                            )
                            .unwrap();
                        }
                    }
                });

                ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut self.generated).font(TextStyle::Monospace),
                );
            });

            ui.separator();

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    egui::warn_if_debug_build(ui);
                });
            });
        });
    }
}
