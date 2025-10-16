use std::fmt::format;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::thread;
use eframe::egui;
use rand::Rng;
use scraper::{Html, Selector};

const BASE_URL: &str = "https://eolymp.com/uk/problems";
const MIN_PROBLEM_ID: u32 = 1;
const MAX_PROBLEM_ID: u32 = 12000;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0]),
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
}

struct MyApp {
    url: String,
    problem_id: Option<u32>,
    name: Option<String>,
    is_loading: bool,
    last_action: Option<AppAction>,
    timestamp: Option<Instant>,
    rx: mpsc::Receiver<String>,
    tx: mpsc::Sender<String>,
}

impl MyApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            url: String::new(),
            problem_id: None,
            name: None,
            is_loading: false,
            last_action: None,
            timestamp: None,
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

    fn get_action_message(&self) -> Option<String> {
        if let (Some(action), Some(timestamp)) = (self.last_action, self.timestamp) {
            if timestamp.elapsed() < Duration::from_secs(1) {
                return Some(match action {
                    AppAction::Generated => "✅ URL згенеровано!".to_string(),
                    AppAction::Opened => "🌐 URL відкрито в браузері!".to_string(),
                    AppAction::Copied => "📋 Скопійовано в буфер обміну!".to_string(),
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
            .show(ctx, |ui| {
                ui.heading("Eolymp");

                ui.separator();
                self.render_main_section(ui, ctx);

                ui.separator();
                self.render_action_feedback(ui, ctx);

                ui.separator();
            });
    }
}

impl MyApp {
    fn render_main_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            if ui.button("🎲 Generate")
                .clicked()
            {
                self.generate_url();
            };

            if ui.add_enabled(self.is_url_valid(), egui::Button::new("🌐 Open"))
                .clicked()
            {
                self.open_url();
            }

            if ui.add_enabled(self.is_url_valid(), egui::Button::new("📋 Copy"))
                .clicked()
            {
                self.copy(ctx);
            }
        });

        ui.separator();

        if self.is_loading {
            ui.label("⏳ Завантаження нової задачі...");
        }
        else if let Some(name) = &self.name {
            ui.colored_label(egui::Color32::LIGHT_BLUE, format!("📝 {}", name));
        }

        ui.label("URL:");
        ui.text_edit_singleline(&mut self.url);

        if let Some(id) = self.problem_id {
            ui.label(format!("Problem ID: {}", id));
        }
    }

    fn render_action_feedback(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(message) = self.get_action_message() {
            ui.colored_label(egui::Color32::GREEN, &message);
        }
    }
}