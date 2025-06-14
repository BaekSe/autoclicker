use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use enigo::{Button, Direction, Enigo, Mouse, Settings, NewConError};
use device_query::{DeviceQuery, DeviceState};

#[derive(Clone)]
pub struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Rect {
    pub fn contains(&self, (x, y): (i32, i32)) -> bool {
        let (min_x, max_x) = if self.x1 < self.x2 { (self.x1, self.x2) } else { (self.x2, self.x1) };
        let (min_y, max_y) = if self.y1 < self.y2 { (self.y1, self.y2) } else { (self.y2, self.y1) };
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }
}

#[derive(Clone)]
pub struct Config {
    pub cps: f32,
    pub repeat: u32, // 0 = infinite
    pub region: Option<Rect>,
}

pub struct AutoClicker {
    config: Arc<Mutex<Config>>,
    running: Arc<AtomicBool>,
}

impl AutoClicker {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn update_config(&self, config: Config) {
        *self.config.lock().unwrap() = config;
    }

    pub fn toggle(&self) {
        if self.is_running() {
            self.stop();
        } else if let Err(e) = self.start() {
            eprintln!("Failed to start AutoClicker: {e}");
        }
    }

    pub fn start(&self) -> Result<(), NewConError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // already running
        }

        let running = self.running.clone();
        let cfg = self.config.lock().unwrap().clone();

        let enigo = match Enigo::new(&Settings::default()) {
            Ok(e) => e,
            Err(e) => {
                self.running.store(false, Ordering::SeqCst);
                return Err(e);
            }
        };

        thread::spawn(move || {
            let mut enigo = enigo;
            let device = DeviceState::new();
            let mut remaining = if cfg.repeat == 0 { None } else { Some(cfg.repeat) };
            while running.load(Ordering::SeqCst) {
                let pos = device.get_mouse().coords;
                if let Some(region) = &cfg.region {
                    if !region.contains(pos) {
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                }
                let _ = enigo.button(Button::Left, Direction::Click);
                if let Some(rem) = remaining.as_mut() {
                    if *rem == 0 {
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                    *rem -= 1;
                }
                let interval = 1.0 / cfg.cps.max(1.0);
                thread::sleep(Duration::from_secs_f32(interval));
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains() {
        let r = Rect { x1: 0, y1: 0, x2: 10, y2: 10 };
        assert!(r.contains((5,5)));
        assert!(!r.contains((15,5)));
    }
}
