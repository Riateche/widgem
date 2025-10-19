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

#[cfg(target_os = "macos")]
pub use crate::macos::{AXUIElementExt, AXValueExt, WindowExt};

pub use enigo::{Button, Key};

use {
    anyhow::Context as _,
    enigo::{Axis, Direction, Enigo, Keyboard, Mouse},
    image::{Rgba, RgbaImage},
    std::{
        sync::{Arc, Mutex},
        thread::sleep,
        time::Duration,
    },
};

struct ContextData {
    imp: imp::Context,
    enigo: Mutex<Enigo>,
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
        })))
    }

    pub fn all_windows(&self) -> anyhow::Result<Vec<Window>> {
        imp::all_windows(self)
    }

    /// Returns windows created by the process `pid`.
    pub fn windows_by_pid(&self, pid: u32) -> anyhow::Result<Vec<Window>> {
        let windows = self.all_windows()?;
        Ok(windows
            .into_iter()
            .filter(|w| w.pid().ok() == Some(pid))
            .collect())
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

#[derive(Clone)]
pub struct Window(imp::Window);

impl Window {
    /// The window id
    pub fn id(&self) -> anyhow::Result<u32> {
        self.0.id()
    }

    pub fn pid(&self) -> anyhow::Result<u32> {
        self.0.pid()
    }

    /// The window app name
    pub fn app_name(&self) -> anyhow::Result<String> {
        self.0.app_name()
    }
    /// The window title
    pub fn title(&self) -> anyhow::Result<String> {
        self.0.title()
    }

    /// The window x coordinate.
    ///
    /// It returns outer position on MacOS, inner position on Linux and Windows.
    pub fn pos_x(&self) -> anyhow::Result<i32> {
        self.0.x()
    }

    /// The window y coordinate.
    ///
    /// It returns outer position on MacOS, inner position on Linux and Windows.
    pub fn y(&self) -> anyhow::Result<i32> {
        self.0.y()
    }

    /// The window pixel width.
    ///
    /// It returns outer size on MacOS, inner size on Linux and Windows.
    pub fn width(&self) -> anyhow::Result<u32> {
        self.0.width()
    }

    /// The window pixel height.
    ///
    /// It returns outer size on MacOS, inner size on Linux and Windows.
    pub fn height(&self) -> anyhow::Result<u32> {
        self.0.height()
    }

    pub fn is_minimized(&self) -> anyhow::Result<bool> {
        self.0.is_minimized()
    }

    pub fn is_maximized(&self) -> anyhow::Result<bool> {
        self.0.is_maximized()
    }

    /// Captures a screenshot of the window.
    ///
    /// On Macos, the image shows the window with system title and frame.
    /// On Linux and Windows, the image shows the window's inner area.
    pub fn capture_image(&self) -> anyhow::Result<RgbaImage> {
        self.0.capture_image()
    }

    pub fn activate(&self) -> anyhow::Result<()> {
        self.0.activate()
    }

    // TODO: move title height hacks out

    /// Move the mouse pointer to the coordinates specified relative to the window's position.
    ///
    /// It uses outer position on MacOS, inner position on Linux and Windows.
    pub fn mouse_move(&self, x: i32, y: i32) -> anyhow::Result<()> {
        self.0.mouse_move(x, y)
    }

    pub fn minimize(&self) -> anyhow::Result<()> {
        self.0.minimize()
    }

    pub fn close(&self) -> anyhow::Result<()> {
        self.0.close()
    }

    /// Change the window's size to the specified values.
    ///
    /// It uses outer size on MacOS, inner position on Linux and Windows.
    pub fn resize(&self, width: i32, height: i32) -> anyhow::Result<()> {
        self.0.resize(width, height)
    }
}
