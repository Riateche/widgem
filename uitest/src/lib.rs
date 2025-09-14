#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
use std::sync::Mutex;

#[cfg(all(unix, not(target_os = "macos")))]
use self::linux as imp;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows as imp;

use enigo::{Axis, Direction, Enigo, Keyboard, Mouse};

mod window;

pub use {
    crate::window::Window,
    enigo::{Button, Key},
};

use {
    anyhow::bail,
    std::{
        sync::Arc,
        thread::sleep,
        time::{Duration, Instant},
    },
};

const SINGLE_WAIT_DURATION: Duration = Duration::from_millis(200);
const DEFAULT_WAIT_DURATION: Duration = Duration::from_secs(5);

struct ContextData {
    imp: imp::Context,
    enigo: Mutex<Enigo>,
    wait_duration: Duration,
}

#[derive(Clone)]
pub struct Context(Arc<ContextData>);

impl Context {
    #[allow(clippy::new_without_default)]
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(Arc::new(ContextData {
            imp: imp::Context::new()?,
            enigo: Mutex::new(Enigo::new(&enigo::Settings::default())?),
            wait_duration: DEFAULT_WAIT_DURATION,
        })))
    }

    pub fn all_windows(&self) -> anyhow::Result<Vec<Window>> {
        xcap::Window::all()?
            .into_iter()
            .map(|w| Window::new(self.clone(), w))
            .collect()
    }

    pub fn windows_by_pid(&self, pid: u32) -> anyhow::Result<Vec<Window>> {
        let windows = self.all_windows()?;
        Ok(windows.into_iter().filter(|w| w.pid() == pid).collect())
    }

    pub fn wait_for_windows_by_pid(
        &self,
        pid: u32,
        num_windows: usize,
    ) -> anyhow::Result<Vec<Window>> {
        let started = Instant::now();
        let mut windows = Vec::new();
        while started.elapsed() < self.0.wait_duration {
            windows = self.windows_by_pid(pid)?;
            if windows.len() == num_windows {
                return Ok(windows);
            }
            sleep(SINGLE_WAIT_DURATION);
        }
        if windows.is_empty() {
            bail!(
                "couldn't find a window with pid={} after {:?}",
                pid,
                self.0.wait_duration
            );
        } else if windows.len() > num_windows {
            bail!(
                "expected to find {} windows with pid={}, but found {} windows",
                num_windows,
                pid,
                windows.len(),
            );
        } else {
            bail!(
                "expected to find {} windows with pid={}, but found only {} windows after {:?}",
                num_windows,
                pid,
                windows.len(),
                self.0.wait_duration
            );
        }
    }

    pub fn active_window_id(&self) -> anyhow::Result<u32> {
        self.0.imp.active_window_id()
    }

    pub fn mouse_click(&self, button: Button) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(button, Direction::Click)?;
        Ok(())
    }

    pub fn mouse_left_click(&self) -> anyhow::Result<()> {
        self.mouse_click(Button::Left)
    }

    pub fn mouse_scroll_up(&self) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().scroll(-1, Axis::Vertical)?;
        Ok(())
    }

    pub fn mouse_scroll_down(&self) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().scroll(1, Axis::Vertical)?;
        Ok(())
    }

    pub fn mouse_scroll_left(&self) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().scroll(-1, Axis::Horizontal)?;
        Ok(())
    }

    pub fn mouse_scroll_right(&self) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().scroll(1, Axis::Horizontal)?;
        Ok(())
    }

    pub fn mouse_down(&self, button: Button) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(button, Direction::Press)?;
        Ok(())
    }

    pub fn mouse_up(&self, button: Button) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(button, Direction::Release)?;
        Ok(())
    }

    pub fn mouse_left_press(&self) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(Button::Left, Direction::Press)?;
        Ok(())
    }

    pub fn mouse_left_release(&self) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(Button::Left, Direction::Release)?;
        Ok(())
    }

    // https://wiki.linuxquestions.org/wiki/List_of_keysyms
    // https://manpages.ubuntu.com/manpages/trusty/man1/xdotool.1.html
    pub fn key(&self, key: Key) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().key(key, Direction::Click)?;
        Ok(())
    }

    pub fn key_combination(&self, keys: &[Key]) -> anyhow::Result<()> {
        for key in keys {
            self.0.enigo.lock().unwrap().key(*key, Direction::Press)?;
        }
        for key in keys.iter().rev() {
            self.0.enigo.lock().unwrap().key(*key, Direction::Release)?;
        }
        Ok(())
    }

    pub fn type_text(&self, text: &str) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().text(text)?;
        Ok(())
    }

    pub fn mouse_move_global(&self, x: i32, y: i32) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .move_mouse(x, y, enigo::Coordinate::Abs)?;
        // self.0.imp.mouse_move_global(x, y)
        Ok(())
    }
}
