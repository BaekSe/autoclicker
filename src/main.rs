use crossbeam_channel::Receiver;
use eframe::{egui, App, Frame, NativeOptions};
use tray_icon::{menu::{Menu, MenuItem, MenuEvent}, TrayIconBuilder, Icon};
use std::time::Duration;

use autoclicker::auto_clicker::{AutoClicker, Config, Rect};

pub struct AutoClickerApp {
    clicker: AutoClicker,
    cps: f32,
    repeat: u32,
    region: Rect,
    rx: Receiver<()>,
}

impl AutoClickerApp {
    fn new(rx: Receiver<()>) -> Self {
        let clicker = AutoClicker::new(Config { cps: 10.0, repeat: 0, region: None });
        Self {
            clicker,
            cps: 10.0,
            repeat: 0,
            region: Rect { x1: 0, y1: 0, x2: 300, y2: 300 },
            rx,
        }
    }

    fn toggle(&mut self) {
        let cfg = Config {
            cps: self.cps,
            repeat: self.repeat,
            region: Some(self.region.clone()),
        };
        self.clicker.update_config(cfg);
        self.clicker.toggle();
    }

    fn capture_region(&mut self) {
        use device_query::{DeviceQuery, DeviceState};
        let device = DeviceState::new();
        println!("Drag to select region with left mouse button");
        // wait for press
        loop {
            let m = device.get_mouse();
            if *m.button_pressed.get(1).unwrap_or(&false) {
                let start = m.coords;
                // wait for release
                loop {
                    let m = device.get_mouse();
                    if !*m.button_pressed.get(1).unwrap_or(&false) {
                        let end = m.coords;
                        self.region = Rect {
                            x1: start.0,
                            y1: start.1,
                            x2: end.0,
                            y2: end.1,
                        };
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl App for AutoClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        while self.rx.try_recv().is_ok() {
            self.toggle();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Auto Clicker");
            ui.add(egui::Slider::new(&mut self.cps, 1.0..=60.0).text("Clicks per second"));
            ui.add(egui::DragValue::new(&mut self.repeat).prefix("Repeat (0=inf): "));
            ui.label("Region (x1, y1, x2, y2)");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.region.x1));
                ui.add(egui::DragValue::new(&mut self.region.y1));
                ui.add(egui::DragValue::new(&mut self.region.x2));
                ui.add(egui::DragValue::new(&mut self.region.y2));
            });
            if ui.button("Set Region by Drag").clicked() {
                self.capture_region();
            }
            let label = if self.clicker.is_running() { "Stop (Cmd+D)" } else { "Start (Cmd+D)" };
            if ui.button(label).clicked() {
                self.toggle();
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let (tx, rx) = crossbeam_channel::unbounded();
    std::thread::spawn(move || {
        use device_query::{DeviceState, Keycode};
        use device_query::DeviceQuery;
        use std::thread; use std::time::Duration;
        let device = DeviceState::new();
        loop {
            let keys = device.get_keys();
            let cmd = keys.contains(&Keycode::Command) || keys.contains(&Keycode::RCommand);
            if cmd && keys.contains(&Keycode::D) {
                let _ = tx.send(());
                thread::sleep(Duration::from_millis(300));
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    // tray icon with quit menu
    let mut tray_menu = Menu::new();
    let quit_item = MenuItem::new("Quit", true, None);
    let _ = tray_menu.append(&quit_item);
    let icon = Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).unwrap();
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("AutoClicker")
        .with_icon(icon)
        .build()
        .unwrap();
    std::thread::spawn(move || {
        let menu_rx = MenuEvent::receiver();
        while let Ok(event) = menu_rx.recv() {
            if event.id == quit_item.id() {
                std::process::exit(0);
            }
        }
    });

    let app = AutoClickerApp::new(rx);
    let opts = NativeOptions::default();
    eframe::run_native("Auto Clicker", opts, Box::new(|_cc| Ok(Box::new(app))))
}
