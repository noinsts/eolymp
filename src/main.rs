mod db;

use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::thread;

use eframe::egui;
use rand::Rng;
use scraper::{Html, Selector};

use crate::db::Database;

const BASE_URL: &str = "https://eolymp.com/uk/problems";
const MIN_PROBLEM_ID: u32 = 1;
const MAX_PROBLEM_ID: u32 = 12000;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Eolymp Problem Generator",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new())))
    )
}

#[derive(Debug, Copy, Clone)]
enum AppAction {
    Generated,
    Opened,
    Copied,
    Saved,
    Deleted,
}

struct MyApp {
    url: String,
    problem_id: Option<u32>,
    name: Option<String>,
    is_loading: bool,
    last_action: Option<AppAction>,
    timestamp: Option<Instant>,
    saved_problems: Vec<db::Problem>,
    db: Database,
    rx: mpsc::Receiver<String>,
    tx: mpsc::Sender<String>,
}

impl MyApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let db = Database::new().expect("Could not initialize database");

        let mut app = Self {
            url: String::new(),
            problem_id: None,
            name: None,
            is_loading: false,
            last_action: None,
            timestamp: None,
            saved_problems: Vec::new(),
            db,
            rx,
            tx,
        };

        app.reload_problems();
        app
    }

    fn generate_url(&mut self) {
        let mut rng = rand::rng();
        let problem_id = rng.random_range(MIN_PROBLEM_ID..=MAX_PROBLEM_ID);

        self.problem_id = Some(problem_id);
        self.url = self.build_url(problem_id);
        self.name = None;
        self.is_loading = true;
        self.set_action(AppAction::Generated);

        self.fetch_title();
    }

    fn build_url(&self, id: u32) -> String {
        format!("{}/{}", BASE_URL, id)
    }

    fn open_url(&mut self, url: String) {
        if let Err(e) = open::that(&url) {
            eprintln!("Помилка при відкритті URL: {}", e);
        }
        self.set_action(AppAction::Opened);
    }

    fn is_url_valid(&self) -> bool {
        self.problem_id.is_some()
    }

    fn copy(&mut self, ctx: &egui::Context, url: String) {
        ctx.copy_text(url);
        self.set_action(AppAction::Copied);
    }

    fn save(&mut self) {
        if let (Some(id), Some(name)) = (self.problem_id, &self.name) {
            match self.db.save_problem(id as i32, name.clone(), self.url.clone()) {
                Ok(_) => {
                    self.set_action(AppAction::Saved);
                    self.reload_problems();
                }
                Err(e) => eprintln!("Помилка при збереженні задачі: {:?}", e),
            }
        }
    }

    fn reload_problems(&mut self) {
        match self.db.get_all_problems() {
            Ok(problems) => self.saved_problems = problems,
            Err(e) => eprintln!("Помилка при завантаженні задач: {:?}", e),
        }
    }

    fn delete_saved_problem(&mut self, id: i32) {
        match self.db.delete_problem(id) {
            Ok(_) => {
                self.set_action(AppAction::Deleted);
                self.reload_problems();
            }
            Err(e) => eprintln!("Помилка видалення задачі: {}", e),
        };
    }

    fn get_action_message(&self) -> Option<String> {
        if let (Some(action), Some(timestamp)) = (self.last_action, self.timestamp) {
            if timestamp.elapsed() < Duration::from_secs(1) {
                return Some(match action {
                    AppAction::Generated => "✅ URL згенеровано!".to_string(),
                    AppAction::Opened => "🌐 URL відкрито в браузері!".to_string(),
                    AppAction::Copied => "📋 Скопійовано в буфер обміну!".to_string(),
                    AppAction::Saved => "💾 Задачу збережено".to_string(),
                    AppAction::Deleted => "🗑 Задачу видалено".to_string(),
                })
            }
        }
        None
    }

    fn set_action(&mut self, action: AppAction) {
        self.last_action = Some(action);
        self.timestamp = Some(Instant::now());
    }

    fn fetch_title(&self) {
        let tx = self.tx.clone();
        let url = self.url.clone();

        thread::spawn(move || {
            if let Some(title) = Self::get_problem_title(&url) {
                let _ = tx.send(title);
            }
        });
    }

    fn get_problem_title(url: &str) -> Option<String> {
        let html = reqwest::blocking::get(url).ok()?.text().ok()?;
        let document = Html::parse_document(&html);

        if let Ok(selector) = Selector::parse("title") {
            if let Some(element) = document.select(&selector).next() {
                let title = element.text().collect::<String>().trim().to_string();
                if !title.is_empty() {
                    return Some(title);
                }
            }
        }
        None
    }

    fn check_for_title(&mut self) {
        if let Ok(title) = self.rx.try_recv() {
            self.name = Some(title);
            self.is_loading = false;
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.check_for_title();

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(15, 15, 15),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(15.0);
                    ui.heading(
                        egui::RichText::new("🔗 Eolymp Problem Generator")
                            .size(28.0)
                    );
                    ui.label(
                        egui::RichText::new("Знайди випадкову задачу для розв'язання")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)),
                    );
                    ui.add_space(20.0);

                    ui.vertical_centered(|ui| {
                        self.render_main_section(ui, ctx);
                        ui.add_space(20.0);
                        self.render_info_section(ui, ctx);
                        ui.add_space(10.0);
                        self.render_action_feedback(ui, ctx);
                    });

                    ui.add_space(25.0);
                    ui.separator();
                    ui.add_space(15.0);

                    self.render_saved_problems(ui, ctx);
                });
            });
    }
}

impl MyApp {
    fn render_main_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let button_width = 110.0;
        let button_height = 45.0;
        let spacing_x = 10.0;
        let num_buttons = 4.0;

        let total_buttons_width = num_buttons * button_width + (num_buttons - 1.0) * spacing_x;
        let left_padding = (ui.available_width() - total_buttons_width) / 2.0;

        ui.horizontal(|ui| {
            ui.add_space(left_padding.max(0.0));
            ui.spacing_mut().item_spacing.x = spacing_x;

            // Generate button
            if ui.add(
                egui::Button::new(
                    egui::RichText::new("🎲 Generate")
                        .size(13.0)
                        .color(egui::Color32::WHITE)
                        .strong()
                )
                    .fill(egui::Color32::from_rgb(200, 100, 255))
                    .min_size(egui::vec2(button_width, button_height))
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(220, 150, 255)))
            )
                .on_hover_text("Натисни щоб згенерувати нову задачу")
                .clicked()
            {
                self.generate_url();
            }

            // Open button
            if ui.add_enabled(
                self.is_url_valid(),
                egui::Button::new(
                    egui::RichText::new("🌐 Open")
                        .size(13.0)
                        .color(egui::Color32::WHITE)
                        .strong()
                )
                    .fill(egui::Color32::from_rgb(100, 200, 150))
                    .min_size(egui::vec2(button_width, button_height))
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(150, 255, 200)))
            )
                .on_hover_text("Відкрити задачу у браузері")
                .clicked()
            {
                self.open_url(self.url.clone());
            }

            // Copy button
            if ui.add_enabled(
                self.is_url_valid(),
                egui::Button::new(
                    egui::RichText::new("📋 Copy")
                        .size(13.0)
                        .color(egui::Color32::WHITE)
                        .strong()
                )
                    .fill(egui::Color32::from_rgb(255, 180, 100))
                    .min_size(egui::vec2(button_width, button_height))
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 150)))
            )
                .on_hover_text("Скопіювати URL у буфер")
                .clicked()
            {
                self.copy(ctx, self.url.clone());
            }

            // Save button
            if ui.add_enabled(
                self.is_url_valid(),
                egui::Button::new(
                    egui::RichText::new("💾 Save")
                        .size(13.0)
                        .color(egui::Color32::WHITE)
                        .strong()
                )
                    .fill(egui::Color32::from_rgb(100, 100, 100))
                    .min_size(egui::vec2(button_width, button_height))
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 150)))
            )
                .on_hover_text("Зберігає задачу")
                .clicked()
            {
                self.save();
            }
        });
    }

    fn render_info_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.group(|ui| {
            ui.set_width(ui.available_width() * 0.8);

            ui.add_space(8.0);

            // Title
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("📝 Назва:")
                        .size(13.0)
                        .strong()
                        .color(egui::Color32::from_rgb(200, 200, 200))
                );

                if self.is_loading {
                    ui.add_space(5.0);
                    ui.spinner();
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        egui::RichText::new("⏳ Завантаження...")
                            .size(13.0)
                    );
                }
                else if let Some(name) = &self.name {
                    ui.colored_label(
                        egui::Color32::from_rgb(150, 200, 255),
                        egui::RichText::new(name).size(12.0)
                    );
                }
                else {
                    ui.colored_label(
                        egui::Color32::DARK_GRAY,
                        egui::RichText::new("(---)").size(12.0)
                    );
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // URL
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("🔗 URL")
                        .size(13.0)
                        .strong()
                        .color(egui::Color32::from_rgb(200, 200, 200))
                );

                if self.url.is_empty() {
                    ui.colored_label(
                        egui::Color32::DARK_GRAY,
                        egui::RichText::new("(---)")
                            .size(12.0)
                    );
                }
                else {
                    ui.colored_label(
                        egui::Color32::from_rgb(150, 200, 150),
                        egui::RichText::new(&self.url)
                            .size(12.0)
                    );
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // ID
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("📌 ID Задачі:")
                        .size(13.0)
                        .strong()
                        .color(egui::Color32::from_rgb(200, 200, 200))
                );

                if let Some(id) = self.problem_id {
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 150, 255),
                        format!("#{}", id)
                    );
                }
                else {
                    ui.colored_label(egui::Color32::DARK_GRAY, "(---)");
                }
            });

            ui.add_space(8.0);
        });
    }

    fn render_action_feedback(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(message) = self.get_action_message() {
            ui.colored_label(
                egui::Color32::GREEN,
                egui::RichText::new(message)
                    .size(13.0)
                    .strong()
            );
        }
    }

    fn render_saved_problems(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(
            egui::RichText::new(format!("💾 Збережені задачі, ({})", self.saved_problems.len()))
                .size(16.0)
                .strong()
        );

        ui.add_space(10.0);

        if self.saved_problems.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading(
                    egui::RichText::new("📭")
                        .size(48.0)
                );
                ui.label(
                    egui::RichText::new("Немає збережених задач")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(100, 100, 100))
                );
                ui.label(
                    egui::RichText::new("Згенеруй задачу та натисни 💾 для збереження")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(80, 80, 80))
                );
                ui.add_space(20.0);
            });
        }
        else {
            let mut to_delete = None;
            let mut to_open = None;
            let mut to_copy = None;
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (idx, problem) in self.saved_problems.iter().enumerate() {
                        ui.group(|ui| {
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.add_space(10.0);

                                // Problem info
                                ui.vertical(|ui| {
                                    // ID
                                    ui.label(
                                        egui::RichText::new(format!("#{}", problem.problem_id))
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(200, 150, 255))
                                            .strong()
                                    );

                                    // Name
                                    ui.label(
                                        egui::RichText::new(&problem.name)
                                            .size(13.0)
                                            .color(egui::Color32::from_rgb(200, 200, 200))
                                            .strong()
                                    );

                                    // URL
                                    ui.label(
                                        egui::RichText::new(&problem.url)
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(100, 150, 200))
                                            .strong()
                                    );
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.add_space(5.0);

                                    // Delete button
                                    if ui.button(
                                        egui::RichText::new("🗑")
                                            .size(16.0)
                                    )
                                        .on_hover_text("Видалити задачу")
                                        .clicked()
                                    {
                                        to_delete = Some(problem.problem_id);
                                    }

                                    ui.add_space(5.0);

                                    // Copy button
                                    if ui.button(
                                        egui::RichText::new("📋")
                                            .size(16.0)
                                    )
                                        .on_hover_text("Копіювати URL")
                                        .clicked()
                                    {
                                        to_copy = Some(problem.url.clone());
                                    }

                                    ui.add_space(5.0);

                                    // Open button
                                    if ui.button(
                                        egui::RichText::new("🔗")
                                            .size(16.0)
                                    )
                                        .on_hover_text("Відкрити в браузері")
                                        .clicked()
                                    {
                                        to_open = Some(problem.url.clone());
                                    }

                                    ui.add_space(10.0);
                                });
                            });

                            ui.add_space(8.0);
                        });

                        if idx < self.saved_problems.len() - 1 {
                            ui.add_space(8.0);
                        }
                    };
                });

            if let Some(id) = to_delete {
                self.delete_saved_problem(id);
            }

            if let Some(url) = to_open {
                self.open_url(url);
            }

            if let Some(url) = to_copy {
                self.copy(ctx, url);
            }
        }
    }
}