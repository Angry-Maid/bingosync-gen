//! Do I care that the code is mess? Kinda no.
//! Do I care that the code is barely readable? Also no.
//! I do kinda know that I did mess around here and it could be done more efficiently.
//! Would I be willing to get help? Yes.

use itertools::Itertools;
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};
use std::{
    borrow::Cow,
    env, fmt,
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};
use weighted_rand::builder::*;

use egui::{FontFamily, FontId, TextStyle, Vec2};
use egui_data_table::RowViewer;
use rfd::FileDialog;

#[derive(PartialEq, Eq)]
enum MainPanel {
    Board,
    Database,
}

impl Default for MainPanel {
    fn default() -> Self {
        Self::Board
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum FieldSize {
    Three = 3,
    Four,
    Five,
}

impl Default for FieldSize {
    fn default() -> Self {
        Self::Five
    }
}

impl fmt::Display for FieldSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

struct CardViewer {
    filter: String,
}

impl Default for CardViewer {
    fn default() -> Self {
        Self {
            filter: Default::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct CardRow(
    #[serde(rename = "category")] String,
    #[serde(rename = "text")] String,
    #[serde(rename = "weight")] f64,
    #[serde(rename = "enabled")] bool,
);

impl RowViewer<CardRow> for CardViewer {
    fn num_columns(&mut self) -> usize {
        4
    }

    fn column_name(&mut self, column: usize) -> Cow<'static, str> {
        ["Category", "Text", "Weight", "Enabled"][column].into()
    }

    fn is_sortable_column(&mut self, column: usize) -> bool {
        [true, true, true, false][column]
    }

    fn create_cell_comparator(&mut self) -> fn(&CardRow, &CardRow, usize) -> std::cmp::Ordering {
        fn cmp(row_l: &CardRow, row_r: &CardRow, column: usize) -> std::cmp::Ordering {
            match column {
                0 => row_l.0.cmp(&row_r.0),
                1 => row_l.1.cmp(&row_r.1),
                2 => row_l.2.partial_cmp(&row_r.2).unwrap(),
                3 => unreachable!(),
                _ => unreachable!(),
            }
        }

        cmp
    }

    fn new_empty_row(&mut self) -> CardRow {
        CardRow(String::from(""), String::from(""), 1.0_f64, true)
    }

    fn set_cell_value(&mut self, src: &CardRow, dst: &mut CardRow, column: usize) {
        match column {
            0 => dst.0 = src.0.clone(),
            1 => dst.1 = src.1.clone(),
            2 => dst.2 = src.2,
            3 => dst.3 = src.3,
            _ => unreachable!(),
        }
    }

    fn show_cell_view(&mut self, ui: &mut egui::Ui, row: &CardRow, column: usize) {
        let _ = match column {
            0 => ui.label(&row.0),
            1 => ui.label(&row.1),
            2 => ui.label(format!("{}", &row.2)),
            3 => ui.checkbox(&mut { row.3 }, ""),
            _ => unreachable!(),
        };
    }

    fn row_filter_hash(&mut self) -> &impl std::hash::Hash {
        &self.filter
    }

    fn create_row_filter(&mut self) -> impl Fn(&CardRow) -> bool {
        |r| r.1.contains(&self.filter)
    }

    fn show_cell_editor(
        &mut self,
        ui: &mut egui::Ui,
        row: &mut CardRow,
        column: usize,
    ) -> Option<egui::Response> {
        match column {
            0 => egui::TextEdit::singleline(&mut row.0).show(ui).response,
            1 => {
                egui::TextEdit::multiline(&mut row.1)
                    .desired_rows(2)
                    .show(ui)
                    .response
            }
            2 => ui.add(
                egui::DragValue::new(&mut row.2)
                    .clamp_range(0.0..=255.0)
                    .speed(1.0),
            ),
            3 => ui.checkbox(&mut row.3, ""),
            _ => unreachable!(),
        }
        .into()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BingoSyncGen {
    #[serde(skip)]
    selected_panel: MainPanel,

    #[serde(skip)]
    board: [Box<String>; 25],

    #[serde(skip)]
    generated: String,

    save_path: PathBuf,

    #[serde(skip)]
    category_select: String,

    #[serde(skip)]
    field_size: FieldSize,

    card_table_data: Vec<CardRow>,

    #[serde(skip)]
    card_table: egui_data_table::DataTable<CardRow>,

    #[serde(skip)]
    card_viewer: CardViewer,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct BingoCard {
    name: String,
}

impl Default for BingoSyncGen {
    fn default() -> Self {
        Self {
            selected_panel: MainPanel::default(),
            board: core::array::from_fn(|_idx| Box::new(String::from(""))),
            generated: String::from(""),
            save_path: env::current_dir().unwrap(),
            category_select: String::from("All"),
            field_size: FieldSize::default(),
            card_table_data: Default::default(),
            card_table: Default::default(),
            card_viewer: CardViewer::default(),
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

        if let Some(storage) = cc.storage {
            let mut value: BingoSyncGen =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();

            value.card_table.extend(value.card_table_data.clone());

            return value;
        }

        Default::default()
    }
}

impl eframe::App for BingoSyncGen {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.card_table_data.clear();
        self.card_table_data
            .extend(self.card_table.iter().map(|item| item.to_owned()));

        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.generated = serde_json::to_string_pretty(&self.board.clone().map(|item| BingoCard {
            name: *item.to_owned(),
        }))
        .unwrap();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if self.selected_panel == MainPanel::Board {
                        if ui.button("Clear Board").clicked() {
                            for i in 0..self.board.len() {
                                *self.board[i] = "".to_owned();
                            }
                            ui.close_menu();
                        }
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.selected_panel, MainPanel::Board, "Bingo Board");
                    ui.selectable_value(&mut self.selected_panel, MainPanel::Database, "Database");
                });
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_panel {
                MainPanel::Board => {
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
                                    .add_filter("JSON", &["json"])
                                    .add_filter("Text File", &["txt"])
                                    .set_directory(&self.save_path)
                                    .save_file();

                                if let Some(path) = save_path {
                                    let file = File::create(path).unwrap();
                                    let mut writer = BufWriter::new(file);
                                    serde_json::to_writer_pretty(
                                        &mut writer,
                                        &self.board.clone().map(|item| BingoCard {
                                            name: *item.to_owned(),
                                        }),
                                    )
                                    .unwrap();
                                }
                            }

                            ui.label("Category".to_owned());

                            egui::ComboBox::from_id_source("category_select")
                                .selected_text(self.category_select.to_owned())
                                .show_ui(ui, |ui| {
                                    for item in [
                                        vec![String::from("All")],
                                        self.card_table
                                            .iter()
                                            .map(|item| item.0.to_owned())
                                            .collect::<Vec<String>>()
                                            .iter()
                                            .unique()
                                            .map(|item| item.to_owned())
                                            .collect::<Vec<String>>(),
                                    ]
                                    .concat()
                                    {
                                        ui.selectable_value(
                                            &mut self.category_select,
                                            item.to_owned(),
                                            item.to_owned(),
                                        );
                                    }
                                });

                            egui::ComboBox::from_id_source("field_select")
                                .selected_text(match self.field_size {
                                    FieldSize::Three => String::from("3x3"),
                                    FieldSize::Four => String::from("4x4"),
                                    FieldSize::Five => String::from("5x5"),
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.field_size,
                                        FieldSize::Three,
                                        "3x3",
                                    );
                                    ui.selectable_value(
                                        &mut self.field_size,
                                        FieldSize::Four,
                                        "4x4",
                                    );
                                    ui.selectable_value(
                                        &mut self.field_size,
                                        FieldSize::Five,
                                        "5x5",
                                    );
                                });

                            if ui.button("Randomize").clicked() {
                                for i in 0..self.board.len() {
                                    *self.board[i] = "".to_owned();
                                }

                                let items_iter = self.card_table.iter();
                                let mut rng = thread_rng();
                                let mut samples: Option<Vec<&CardRow>> = None;

                                if self.category_select.ne("All") {
                                    samples = items_iter
                                        .filter(|i| {
                                            i.0.to_owned().eq(&self.category_select.to_owned())
                                                && i.3
                                        })
                                        .choose_multiple(
                                            &mut rng,
                                            self.field_size as usize * self.field_size as usize,
                                        )
                                        .into();
                                } else {
                                    samples = items_iter
                                        .filter(|i| i.3)
                                        .choose_multiple(
                                            &mut rng,
                                            self.field_size as usize * self.field_size as usize,
                                        )
                                        .into();
                                }

                                let mut result: Vec<&CardRow> =
                                    samples.as_slice().first().unwrap().to_owned();
                                result.shuffle(&mut rng);

                                match self.field_size {
                                    FieldSize::Three => {
                                        let mut consume = result.iter();
                                        for c in 1..=3 {
                                            for r in 1..=3 {
                                                *self.board[c * 5 + r] =
                                                    consume.next().unwrap().1.to_owned();
                                            }
                                        }
                                    }
                                    FieldSize::Four => {
                                        let mut consume = result.iter();
                                        for c in 0..=3 {
                                            for r in 0..=3 {
                                                *self.board[c * 5 + r] =
                                                    consume.next().unwrap().1.to_owned();
                                            }
                                        }
                                    }
                                    FieldSize::Five => {
                                        for (idx, s) in result.iter().enumerate() {
                                            *self.board[idx] = s.1.to_owned();
                                        }
                                    }
                                }
                            }
                            if ui.button("W. Randomize").clicked() {
                                for i in 0..self.board.len() {
                                    *self.board[i] = "".to_owned();
                                }

                                let items_iter = self.card_table.iter();
                                let mut rng = thread_rng();
                                let mut samples: Option<Vec<&CardRow>> = None;

                                if self.category_select.ne("All") {
                                    samples = items_iter
                                        .filter(|i| {
                                            i.0.to_owned().eq(&self.category_select.to_owned())
                                                && i.3
                                        })
                                        .collect::<Vec<&CardRow>>()
                                        .into();
                                } else {
                                    samples = items_iter
                                        .filter(|i| i.3)
                                        .collect::<Vec<&CardRow>>()
                                        .into();
                                }

                                let result = samples.unwrap();
                                let builder = WalkerTableBuilder::new(
                                    &result
                                        .clone()
                                        .iter()
                                        .map(|item| item.2 as f32 / 100.0)
                                        .collect::<Vec<f32>>(),
                                );
                                let wa_table = builder.build();
                                let mut visited: Vec<usize> = vec![];

                                match self.field_size {
                                    FieldSize::Three => {
                                        for c in 1..=3 {
                                            for r in 1..=3 {
                                                while let idx = wa_table.next_rng(&mut rng) {
                                                    if visited.contains(&idx) {
                                                        continue;
                                                    }

                                                    visited.push(idx);

                                                    *self.board[c * 5 + r] =
                                                        result[idx].1.to_owned();

                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    FieldSize::Four => {
                                        for c in 0..=3 {
                                            for r in 0..=3 {
                                                while let idx = wa_table.next_rng(&mut rng) {
                                                    if visited.contains(&idx) {
                                                        continue;
                                                    }

                                                    visited.push(idx);

                                                    *self.board[c * 5 + r] =
                                                        result[idx].1.to_owned();

                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    FieldSize::Five => {
                                        for e_idx in (0..25) {
                                            while let idx = wa_table.next_rng(&mut rng) {
                                                if visited.contains(&idx) {
                                                    continue;
                                                }

                                                visited.push(idx);

                                                *self.board[e_idx] = result[idx].1.to_owned();

                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        });

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add_sized(
                                ui.available_size(),
                                egui::TextEdit::multiline(&mut self.generated)
                                    .font(TextStyle::Monospace),
                            );
                        });
                    });
                }
                MainPanel::Database => {
                    ui.horizontal(|ui| {
                        ui.label("Search");
                        ui.text_edit_singleline(&mut self.card_viewer.filter);
                        if ui.button("Add Row").clicked() {
                            self.card_table.extend([self.card_viewer.new_empty_row()]);
                        }
                        if ui.button("Import Add").clicked() {
                            let save_path = FileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .set_directory(&self.save_path)
                                .pick_file();

                            if let Some(path) = save_path {
                                let file = File::open(path).unwrap();
                                let reader = BufReader::new(file);
                                let mut dataset = csv::Reader::from_reader(reader);

                                let mut data: Vec<CardRow> = Vec::new();

                                for result in dataset.deserialize() {
                                    let record: CardRow = result.unwrap();
                                    data.push(record);
                                }

                                self.card_table.extend(data);
                            }
                        }
                        if ui.button("Import").clicked() {
                            let save_path = FileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .set_directory(&self.save_path)
                                .pick_file();

                            if let Some(path) = save_path {
                                let file = File::open(path).unwrap();
                                let reader = BufReader::new(file);
                                let mut dataset = csv::Reader::from_reader(reader);

                                let mut data: Vec<CardRow> = Vec::new();

                                for result in dataset.deserialize() {
                                    let record: CardRow = result.unwrap();
                                    data.push(record);
                                }

                                self.card_table.replace(data);
                            }
                        }
                        if ui.button("Export").clicked() {
                            let save_path = FileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .set_directory(&self.save_path)
                                .save_file();

                            if let Some(path) = save_path {
                                let mut writer = csv::Writer::from_path(path).unwrap();

                                writer
                                    .write_record(&["category", "text", "weight", "enabled"])
                                    .unwrap(); // Header

                                for record in self.card_table.iter() {
                                    writer.serialize(record).unwrap();
                                }
                            }
                        }
                    });

                    ui.add(egui_data_table::Renderer::new(
                        &mut self.card_table,
                        &mut self.card_viewer,
                    ));
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    egui::warn_if_debug_build(ui);
                });
            });
        });
    }
}
