use eframe::egui;
use rand::Rng;

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

struct MyApp {
    url: String,
    problem_id: Option<u32>,
}

impl MyApp {
    fn new() -> Self {
        Self {
            url: String::new(),
            problem_id: None,
        }
    }

    fn generate_url(&mut self) {
        let mut rng = rand::rng();
        let problem_id = rng.random_range(MIN_PROBLEM_ID..=MAX_PROBLEM_ID);

        self.problem_id = Some(problem_id);
        self.url = self.build_url(problem_id);
    }

    fn build_url(&self, id: u32) -> String {
        format!("{}/{}", BASE_URL, id)
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.heading("Eolymp");
                ui.separator();

                self.render_main_section(ui, ctx);
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
        });

        ui.label("URL:");
        ui.text_edit_singleline(&mut self.url);

        if let Some(id) = self.problem_id {
            ui.label(format!("Problem ID: {}", id));
        }
    }
}