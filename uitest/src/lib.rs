#[cfg(all(unix, not(target_os = "macos")))]
mod linux;

#[cfg(all(unix, not(target_os = "macos")))]
use crate::linux as imp;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use crate::windows as imp;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
use crate::macos as imp;

#[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
mod xcap_window;

#[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
pub use crate::xcap_window::Window;

#[cfg(target_os = "macos")]
pub use crate::macos::Window;

use anyhow::Context as _;
pub use enigo::{Button, Key};
use image::{Rgba, RgbaImage};

use {
    anyhow::bail,
    enigo::{Axis, Direction, Enigo, Keyboard, Mouse},
    std::{
        sync::{Arc, Mutex},
        thread::sleep,
        time::{Duration, Instant},
    },
};

const SINGLE_WAIT_DURATION: Duration = Duration::from_millis(200);
const DEFAULT_WAIT_DURATION: Duration = Duration::from_secs(15);

struct ContextData {
    imp: imp::Context,
    enigo: Mutex<Enigo>,
    wait_duration: Duration,
}

// Placeholder for a pixel value that is not available because it was
// obscured by a MacOS system window frame.
pub const IGNORED_PIXEL: Rgba<u8> = Rgba([255, 0, 255, 255]);

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
        self.0.imp.all_windows(self)
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
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_left_click(&self) -> anyhow::Result<()> {
        self.mouse_click(Button::Left)
    }

    pub fn mouse_scroll_up(&self) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().scroll(-1, Axis::Vertical)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_scroll_down(&self) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().scroll(1, Axis::Vertical)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_scroll_left(&self) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().scroll(-1, Axis::Horizontal)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_scroll_right(&self) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().scroll(1, Axis::Horizontal)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_down(&self, button: Button) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(button, Direction::Press)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_up(&self, button: Button) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(button, Direction::Release)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_left_press(&self) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(Button::Left, Direction::Press)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_left_release(&self) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .button(Button::Left, Direction::Release)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    // https://wiki.linuxquestions.org/wiki/List_of_keysyms
    // https://manpages.ubuntu.com/manpages/trusty/man1/xdotool.1.html
    pub fn key(&self, key: Key) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().key(key, Direction::Click)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn key_combination(&self, keys: &[Key]) -> anyhow::Result<()> {
        for key in keys {
            self.0.enigo.lock().unwrap().key(*key, Direction::Press)?;
        }
        for key in keys.iter().rev() {
            self.0.enigo.lock().unwrap().key(*key, Direction::Release)?;
        }
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn type_text(&self, text: &str) -> anyhow::Result<()> {
        self.0.enigo.lock().unwrap().text(text)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn mouse_move_global(&self, x: i32, y: i32) -> anyhow::Result<()> {
        self.0
            .enigo
            .lock()
            .unwrap()
            .move_mouse(x, y, enigo::Coordinate::Abs)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn capture_full_screen(&self) -> anyhow::Result<RgbaImage> {
        let image = xcap::Monitor::all()?
            .first()
            .context("no monitors found")?
            .capture_image()?;
        Ok(image)
    }
}
