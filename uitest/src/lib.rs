mod linux;

use self::linux as imp;

use {
    anyhow::bail,
    std::{
        sync::Arc,
        thread::sleep,
        time::{Duration, Instant},
    },
    xcap::{image::RgbaImage, XCapResult},
};

const SINGLE_WAIT_DURATION: Duration = Duration::from_millis(200);
const DEFAULT_WAIT_DURATION: Duration = Duration::from_secs(5);

struct ConnectionInner {
    imp: imp::Context,
    wait_duration: Duration,
}

#[derive(Clone)]
pub struct Connection(Arc<ConnectionInner>);

impl Connection {
    #[allow(clippy::new_without_default)]
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(Arc::new(ConnectionInner {
            imp: imp::Context::new()?,
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
        Ok(windows.into_iter().filter(|w| w.pid == pid).collect())
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

    pub fn mouse_click(&self, button: u32) -> anyhow::Result<()> {
        self.0.imp.mouse_click(button)
    }

    pub fn mouse_scroll_up(&self) -> anyhow::Result<()> {
        self.mouse_click(4)
    }

    pub fn mouse_scroll_down(&self) -> anyhow::Result<()> {
        self.mouse_click(5)
    }

    pub fn mouse_scroll_left(&self) -> anyhow::Result<()> {
        self.mouse_click(6)
    }

    pub fn mouse_scroll_right(&self) -> anyhow::Result<()> {
        self.mouse_click(7)
    }

    pub fn mouse_down(&self, button: u32) -> anyhow::Result<()> {
        self.0.imp.mouse_down(button)
    }

    pub fn mouse_up(&self, button: u32) -> anyhow::Result<()> {
        self.0.imp.mouse_up(button)
    }

    // https://wiki.linuxquestions.org/wiki/List_of_keysyms
    // https://manpages.ubuntu.com/manpages/trusty/man1/xdotool.1.html
    pub fn key(&self, key: &str) -> anyhow::Result<()> {
        self.0.imp.key(key)
    }

    pub fn type_text(&self, text: &str) -> anyhow::Result<()> {
        self.0.imp.type_text(text)
    }

    pub fn mouse_move_global(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.0.imp.mouse_move_global(x, y)
    }
}

#[derive(Clone)]
pub struct Window {
    id: u32,
    pid: u32,
    inner: xcap::Window,
    #[allow(dead_code)]
    connection: Connection,
}

impl Window {
    fn new(connection: Connection, inner: xcap::Window) -> anyhow::Result<Self> {
        Ok(Self {
            id: inner.id()?,
            pid: inner.pid()?,
            inner,
            connection,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// The window id
    pub fn id(&self) -> u32 {
        self.id
    }
    /// The window app name
    pub fn app_name(&self) -> XCapResult<String> {
        self.inner.app_name()
    }
    /// The window title
    pub fn title(&self) -> XCapResult<String> {
        self.inner.title()
    }
    /// The window x coordinate.
    pub fn x(&self) -> XCapResult<i32> {
        self.inner.x()
    }
    /// The window x coordinate.
    pub fn y(&self) -> XCapResult<i32> {
        self.inner.y()
    }
    /// The window pixel width.
    pub fn width(&self) -> XCapResult<u32> {
        self.inner.width()
    }
    /// The window pixel height.
    pub fn height(&self) -> XCapResult<u32> {
        self.inner.height()
    }
    /// The window is minimized.
    pub fn is_minimized(&self) -> XCapResult<bool> {
        self.inner.is_minimized()
    }
    /// The window is maximized.
    pub fn is_maximized(&self) -> XCapResult<bool> {
        self.inner.is_maximized()
    }

    pub fn capture_image(&self) -> anyhow::Result<RgbaImage> {
        Ok(self.inner.capture_image()?)
    }

    pub fn activate(&self) -> anyhow::Result<()> {
        self.connection.0.imp.activate_window(self)
    }

    pub fn mouse_move(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.connection.0.imp.mouse_move(self, x, y)
    }

    pub fn minimize(&self) -> anyhow::Result<()> {
        self.connection.0.imp.minimize_window(self)
    }

    pub fn close(&self) -> anyhow::Result<()> {
        self.connection.0.imp.close_window(self)
    }

    pub fn resize(&self, width: i32, height: i32) -> anyhow::Result<()> {
        self.connection.0.imp.resize_window(self, width, height)
    }
}
