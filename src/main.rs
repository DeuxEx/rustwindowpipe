use eframe::egui;
use std::env;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::thread;



static PIPE_PATH: &str = "/tmp/rust_gui_pipe";



fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();

    if !args.contains(&"--daemon".to_string()) {
        let current_exe = env::current_exe().expect("Couldnt find binary");

        Command::new(current_exe)
            .arg("--daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Couldnt start process in the background");

        return Ok(());
    }


    let (tx, rx) = channel::<String>();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("KeepOnTopWindow")
            .with_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rust GUI",
        options,
        Box::new(|cc| {
            // Hämta en klon av egui::Context för att kunna väcka GUI-tråden från bakgrunden
            let ctx = cc.egui_ctx.clone();

            // Starta tråden med tillgång till både tx och ctx
            thread::spawn(move || {
                let _ = Command::new("mkfifo").arg(PIPE_PATH).status();

                loop {
                    if let Ok(file) = OpenOptions::new().read(true).open(PIPE_PATH) {
                        let reader = BufReader::new(file);
                        for line in reader.lines().flatten() {
                            // 1. Skicka raden till kanalen
                            let _ = tx.send(line);
                            // 2. VÄCK GUI-TRÅDEN DIREKT!
                            ctx.request_repaint();
                        }
                    }
                }
            });

            Box::new(MyApp::new(rx))
        }),
    )
}



struct MyApp {
    rx: Receiver<String>,
    logs: Vec<String>,
}



impl MyApp {
    fn new(rx: Receiver<String>) -> Self {
        Self {
            rx,
            logs: Vec::new(),
        }
    }
}



impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Töm mpsc-kanalen på alla inkomna meddelanden
        while let Ok(line) = self.rx.try_recv() {
            self.logs.push(line);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Live Logg-output");
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.logs {
                        let text_color = if line.contains("ERROR") {
                            egui::Color32::from_rgb(255, 100, 100)
                        } else if line.contains("WARN") {
                            egui::Color32::from_rgb(255, 200, 100)
                        } else if line.contains("INFO") {
                            egui::Color32::from_rgb(100, 200, 255)
                        } else {
                            ui.style().visuals.text_color()
                        };

                        ui.label(
                            egui::RichText::new(line)
                                .monospace()
                                .color(text_color),
                        );
                    }
                });
        });
    }
}
