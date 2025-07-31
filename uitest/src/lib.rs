use {
    anyhow::{bail, Context},
    std::{
        process::Command,
        thread::sleep,
        time::{Duration, Instant},
    },
    x11rb::{
        protocol::xproto::{Atom, ConnectionExt},
        rust_connection::RustConnection,
    },
    xcap::{image::RgbaImage, XCapResult},
};

const SINGLE_WAIT_DURATION: Duration = Duration::from_millis(200);
const DEFAULT_WAIT_DURATION: Duration = Duration::from_secs(5);

pub struct Connection {
    connection: RustConnection,
    net_wm_pid: Atom,
    cardinal: Atom,
    wait_duration: Duration,
}

fn get_or_intern_atom(conn: &RustConnection, name: &[u8]) -> Atom {
    let result = conn
        .intern_atom(false, name)
        .expect("Failed to intern atom")
        .reply()
        .expect("Failed receive interned atom");

    result.atom
}

impl Connection {
    #[allow(clippy::new_without_default)]
    pub fn new() -> anyhow::Result<Self> {
        let (connection, _screen_num) = x11rb::connect(None)?;
        let net_wm_pid = get_or_intern_atom(&connection, b"_NET_WM_PID");
        let cardinal = get_or_intern_atom(&connection, b"CARDINAL");
        Ok(Self {
            connection,
            net_wm_pid,
            cardinal,
            wait_duration: DEFAULT_WAIT_DURATION,
        })
    }

    pub fn all_windows(&self) -> anyhow::Result<Vec<Window>> {
        xcap::Window::all()?
            .into_iter()
            .map(|w| Window::new(self, w))
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
        while started.elapsed() < self.wait_duration {
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
                self.wait_duration
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
                self.wait_duration
            );
        }
    }

    pub fn active_window_id(&self) -> anyhow::Result<u32> {
        let output = Command::new("xdotool")
            .arg("getactivewindow")
            .output()
            .with_context(|| "failed to execute command: xdotool getactivewindow")?;
        if !output.status.success() {
            bail!("xdotool failed: {:?}", output);
        }
        Ok(String::from_utf8(output.stdout)?.trim().parse()?)
    }

    fn run_xdotool(&self, args: &[&str]) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .args(args)
            .status()
            .with_context(|| format!("failed to execute command: xdotool {:?}", args))?;
        if !status.success() {
            bail!("xdotool failed with status {:?}", status);
        }
        Ok(())
    }

    pub fn mouse_click(&self, button: u32) -> anyhow::Result<()> {
        self.run_xdotool(&["click", &button.to_string()])
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
        self.run_xdotool(&["mousedown", &button.to_string()])
    }

    pub fn mouse_up(&self, button: u32) -> anyhow::Result<()> {
        self.run_xdotool(&["mouseup", &button.to_string()])
    }

    // https://wiki.linuxquestions.org/wiki/List_of_keysyms
    // https://manpages.ubuntu.com/manpages/trusty/man1/xdotool.1.html
    pub fn key(&self, key: &str) -> anyhow::Result<()> {
        self.run_xdotool(&["key", key])
    }

    pub fn type_text(&self, text: &str) -> anyhow::Result<()> {
        self.run_xdotool(&["type", text])
    }

    pub fn mouse_move_global(&self, x: u32, y: u32) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("mousemove")
            .arg("--sync")
            .arg(x.to_string())
            .arg(y.to_string())
            .status()?;
        if !status.success() {
            bail!("xdotool failed: {:?}", status);
        }
        Ok(())
    }
}

pub struct Window {
    id: u32,
    pid: u32,
    inner: xcap::Window,
}

impl Window {
    fn new(connection: &Connection, inner: xcap::Window) -> anyhow::Result<Self> {
        let pid = connection
            .connection
            .get_property(
                false,
                inner.id()?,
                connection.net_wm_pid,
                connection.cardinal,
                0,
                u32::MAX,
            )?
            .reply()?
            .value32()
            .unwrap()
            .next()
            .unwrap();
        let id = inner.id()?;
        Ok(Self { id, pid, inner })
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

    pub fn capture_image(&mut self) -> anyhow::Result<RgbaImage> {
        Ok(self.inner.capture_image()?)
    }

    pub fn activate(&self) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("windowactivate")
            .arg("--sync")
            .arg(self.id().to_string())
            .status()?;
        if !status.success() {
            bail!("xdotool failed: {:?}", status);
        }

        // let status = Command::new("xdotool")
        //     .arg("windowraise")
        //     .arg(self.id().to_string())
        //     .status()?;
        // if !status.success() {
        //     bail!("xdotool failed: {:?}", status);
        // }
        Ok(())
    }

    pub fn mouse_move(&self, x: u32, y: u32) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("mousemove")
            .arg("--window")
            .arg(self.id().to_string())
            .arg("--sync")
            .arg(x.to_string())
            .arg(y.to_string())
            .status()?;
        if !status.success() {
            bail!("xdotool failed: {:?}", status);
        }
        Ok(())
    }

    pub fn minimize(&self) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("windowminimize")
            .arg("--sync")
            .arg(self.id().to_string())
            .status()?;
        if !status.success() {
            bail!("xdotool failed: {:?}", status);
        }
        Ok(())
    }

    pub fn close(&self) -> anyhow::Result<()> {
        // `xdotool windowclose` doesn't work properly
        let status = Command::new("wmctrl")
            .arg("-i")
            .arg("-c")
            .arg(self.id().to_string())
            .status()?;
        if !status.success() {
            bail!("wmctrl failed: {:?}", status);
        }
        Ok(())
    }

    pub fn resize(&self, width: i32, height: i32) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("windowsize")
            .arg("--sync")
            .arg(self.id().to_string())
            .arg(width.to_string())
            .arg(height.to_string())
            .status()?;
        if !status.success() {
            bail!("xdotool failed: {:?}", status);
        }
        Ok(())
    }
}
