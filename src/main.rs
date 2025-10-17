mod db;

use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::thread;
use diesel::serialize::ToSql;
use eframe::egui;
use rand::Rng;
use scraper::{Html, Selector};
use crate::db::{Database, Problem};

const BASE_URL: &str = "https://eolymp.com/uk/problems";
const MIN_PROBLEM_ID: u32 = 1;
const MAX_PROBLEM_ID: u32 = 12000;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 400.0]),
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

        Self {
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
        }
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

    fn open_url(&mut self) {
        if let Err(e) = open::that(&self.url) {
            eprintln!("Помилка при відкритті URL: {}", e);
        }
        self.set_action(AppAction::Opened);
    }

    fn is_url_valid(&self) -> bool {
        self.problem_id.is_some()
    }

    fn copy(&mut self, ctx: &egui::Context) {
        ctx.copy_text(self.url.clone());
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

    fn get_action_message(&self) -> Option<String> {
        if let (Some(action), Some(timestamp)) = (self.last_action, self.timestamp) {
            if timestamp.elapsed() < Duration::from_secs(1) {
                return Some(match action {
                    AppAction::Generated => "✅ URL згенеровано!".to_string(),
                    AppAction::Opened => "🌐 URL відкрито в браузері!".to_string(),
                    AppAction::Copied => "📋 Скопійовано в буфер обміну!".to_string(),
                    AppAction::Saved => "💾 Задачу збережено".to_string(),
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
        self.reload_problems();

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(15, 15, 15),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.heading("🔗 Eolymp Problem Generator");
                    ui.add_space(5.0);
                    ui.label("Знайди випадкову задачу для розв'язання");
                    ui.add_space(15.0);

                    self.render_main_section(ui, ctx);

                    ui.add_space(15.0);
                    self.render_info_section(ui, ctx);

                    ui.add_space(15.0);
                    self.render_action_feedback(ui, ctx);

                    ui.add_space(15.0);
                    self.render_saved_problems(ui, ctx);
                });
            });
    }
}

impl MyApp {
    fn render_main_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let button_width = 100.0;
        let button_height = 40.0;
        let spacing_x = 10.0;
        let num_buttons = 4.0;

        let total_buttons_width = num_buttons * button_width + (num_buttons - 1.0) * spacing_x;
        let left_padding = (ui.available_width() - total_buttons_width) / 2.0;

        ui.horizontal(|ui| {
            ui.add_space(left_padding.max(0.0));
            ui.spacing_mut().item_spacing.x = spacing_x;

            if ui.add(
                egui::Button::new(
                    egui::RichText::new("🎲 Generate")
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

            if ui.add_enabled(
                self.is_url_valid(),
                egui::Button::new(
                    egui::RichText::new("🌐 Open")
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
                self.open_url();
            }

            if ui.add_enabled(
                self.is_url_valid(),
                egui::Button::new(
                    egui::RichText::new("📋 Copy")
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
                self.copy(ctx);
            }

            if ui.add_enabled(
                self.is_url_valid(),
                egui::Button::new(
                    egui::RichText::new("💾 Save")
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
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("📝 Назва:").size(14.0).strong()
                );

                if self.is_loading {
                    ui.spinner().on_hover_text("Завантаження даних з серверу...");
                    ui.colored_label(egui::Color32::YELLOW, "⏳ Завантаження...");
                }
                else if let Some(name) = &self.name {
                    ui.colored_label(egui::Color32::from_rgb(150, 200, 255), format!("📝 {}", name));
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("🔗 URL: {}", self.url)).size(14.0).strong()
                );
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("📌 ID:").size(14.0).strong()
                );

                if let Some(id) = self.problem_id {
                    ui.colored_label(egui::Color32::from_rgb(200, 150, 255), format!("#{}", id));
                }
                else {
                    ui.colored_label(egui::Color32::DARK_GRAY, "(---)");
                }
            });
        });
    }

    fn render_action_feedback(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(message) = self.get_action_message() {
            ui.colored_label(egui::Color32::GREEN, &message);
        }
    }

    fn render_saved_problems(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(
            egui::RichText::new("💾 Збережені задачі")
                .size(16.0)
                .strong()
        );

        if self.saved_problems.is_empty() {
            ui.colored_label(egui::Color32::DARK_GRAY, "Немає збережених задач.");
        }
        else {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.group(|ui| {
                        for problem in &self.saved_problems {
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "#{} - {}",
                                    problem.problem_id, problem.name
                                ));

                                if ui.button("🔗").clicked() {

                                }

                                if ui.button("📋").clicked() {

                                }

                                if ui.button("🗑").clicked() {

                                }
                            });
                            ui.separator();
                        };
                    });
                });
        }
    }
}