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

    pub fn activate_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("windowactivate")
            .arg("--sync")
            .arg(window.id().to_string())
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
