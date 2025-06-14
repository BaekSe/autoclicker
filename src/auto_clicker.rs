use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use enigo::{Button, Direction, Enigo, Mouse, Settings};
use device_query::{DeviceQuery, DeviceState};

pub trait ClickEnv {
    fn mouse_coords(&self) -> (i32, i32);
    fn click(&mut self);
    fn sleep(&mut self, d: Duration);
}

struct RealEnv {
    enigo: Enigo,
    device: DeviceState,
}

impl RealEnv {
    fn new() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default()).unwrap(),
            device: DeviceState::new(),
        }
    }
}

impl ClickEnv for RealEnv {
    fn mouse_coords(&self) -> (i32, i32) {
        self.device.get_mouse().coords
    }

    fn click(&mut self) {
        let _ = self.enigo.button(Button::Left, Direction::Click);
    }

    fn sleep(&mut self, d: Duration) {
        thread::sleep(d);
    }
}

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
        } else {
            self.start();
        }
    }

    pub fn start(&self) {
        let _ = self.start_with_env(RealEnv::new());
    }

    pub fn start_with_env<E: ClickEnv + Send + 'static>(&self, env: E) -> Option<thread::JoinHandle<()>> {
        if self.running.swap(true, Ordering::SeqCst) {
            return None; // already running
        }
        let running = self.running.clone();
        let cfg = self.config.lock().unwrap().clone();
        Some(thread::spawn(move || {
            let mut env = env;
            Self::run_loop(cfg, running, &mut env);
        }))
    }

    pub fn run_loop<E: ClickEnv>(cfg: Config, running: Arc<AtomicBool>, env: &mut E) {
        let mut remaining = if cfg.repeat == 0 { None } else { Some(cfg.repeat) };
        while running.load(Ordering::SeqCst) {
            let pos = env.mouse_coords();
            if let Some(region) = &cfg.region {
                if !region.contains(pos) {
                    running.store(false, Ordering::SeqCst);
                    break;
                }
            }
            env.click();
            if let Some(rem) = remaining.as_mut() {
                if *rem == 0 {
                    running.store(false, Ordering::SeqCst);
                    break;
                }
                *rem -= 1;
            }
            let interval = 1.0 / cfg.cps.max(1.0);
            env.sleep(Duration::from_secs_f32(interval));
        }
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

    struct MockEnv {
        positions: Vec<(i32, i32)>,
        idx: usize,
        clicks: usize,
    }

    impl MockEnv {
        fn new(positions: Vec<(i32, i32)>) -> Self {
            Self { positions, idx: 0, clicks: 0 }
        }
    }

    impl ClickEnv for MockEnv {
        fn mouse_coords(&self) -> (i32, i32) {
            self.positions.get(self.idx).copied().unwrap_or_else(|| *self.positions.last().unwrap())
        }

        fn click(&mut self) {
            self.clicks += 1;
            if self.idx < self.positions.len() {
                self.idx += 1;
            }
        }

        fn sleep(&mut self, _d: Duration) {}
    }

    #[test]
    fn stops_when_cursor_leaves_region() {
        let cfg = Config { cps: 10.0, repeat: 0, region: Some(Rect { x1: 0, y1: 0, x2: 10, y2: 10 }) };
        let running = Arc::new(AtomicBool::new(true));
        let mut env = MockEnv::new(vec![(5,5), (20,20)]);
        AutoClicker::run_loop(cfg, running.clone(), &mut env);
        assert!(!running.load(Ordering::SeqCst));
        assert_eq!(env.clicks, 1);
    }

    #[test]
    fn stops_when_repeat_reached() {
        let cfg = Config { cps: 10.0, repeat: 2, region: None };
        let running = Arc::new(AtomicBool::new(true));
        let mut env = MockEnv::new(vec![(0,0); 3]);
        AutoClicker::run_loop(cfg, running.clone(), &mut env);
        assert!(!running.load(Ordering::SeqCst));
        assert_eq!(env.clicks, 2);
    }
}
