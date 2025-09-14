mod linux;

use self::linux as imp;

mod window;

pub use crate::window::Window;

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
    wait_duration: Duration,
}

#[derive(Clone)]
pub struct Context(Arc<ContextData>);

impl Context {
    #[allow(clippy::new_without_default)]
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(Arc::new(ContextData {
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
