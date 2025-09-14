use std::process::Command;

use anyhow::{bail, Context as _};
use x11rb::{
    protocol::xproto::{Atom, ConnectionExt},
    rust_connection::RustConnection,
};

pub struct Context {
    #[allow(dead_code)]
    connection: RustConnection,
    #[allow(dead_code)]
    cardinal: Atom,
}

fn get_or_intern_atom(conn: &RustConnection, name: &[u8]) -> Atom {
    let result = conn
        .intern_atom(false, name)
        .expect("Failed to intern atom")
        .reply()
        .expect("Failed receive interned atom");

    result.atom
}

impl Context {
    pub fn new() -> anyhow::Result<Self> {
        let (connection, _screen_num) = x11rb::connect(None)?;
        let cardinal = get_or_intern_atom(&connection, b"CARDINAL");
        Ok(Self {
            connection,
            cardinal,
        })
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

    pub fn mouse_click(&self, button: u32) -> anyhow::Result<()> {
        self.run_xdotool(&["click", &button.to_string()])
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

    pub fn activate_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("windowactivate")
            .arg("--sync")
            .arg(window.id().to_string())
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

    pub fn mouse_move(&self, window: &crate::Window, x: u32, y: u32) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("mousemove")
            .arg("--window")
            .arg(window.id().to_string())
            .arg("--sync")
            .arg(x.to_string())
            .arg(y.to_string())
            .status()?;
        if !status.success() {
            bail!("xdotool failed: {:?}", status);
        }
        Ok(())
    }

    pub fn minimize_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("windowminimize")
            .arg("--sync")
            .arg(window.id().to_string())
            .status()?;
        if !status.success() {
            bail!("xdotool failed: {:?}", status);
        }
        Ok(())
    }

    pub fn close_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        // `xdotool windowclose` doesn't work properly
        let status = Command::new("wmctrl")
            .arg("-i")
            .arg("-c")
            .arg(window.id().to_string())
            .status()?;
        if !status.success() {
            bail!("wmctrl failed: {:?}", status);
        }
        Ok(())
    }

    pub fn resize_window(
        &self,
        window: &crate::Window,
        width: i32,
        height: i32,
    ) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("windowsize")
            .arg("--sync")
            .arg(window.id().to_string())
            .arg(width.to_string())
            .arg(height.to_string())
            .status()?;
        if !status.success() {
            bail!("xdotool failed: {:?}", status);
        }
        Ok(())
    }
}
