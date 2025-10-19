use {
    anyhow::{bail, Context as _},
    image::RgbaImage,
    std::process::Command,
};

#[derive(Clone)]
pub struct Window {
    window: xcap::Window,
    context: crate::Context,
}

pub fn all_windows(context: &crate::Context) -> anyhow::Result<Vec<crate::Window>> {
    Ok(xcap::Window::all()?
        .into_iter()
        .map(|window| {
            crate::Window(Window {
                window,
                context: context.clone(),
            })
        })
        .collect())
}

pub struct Context {}

impl Context {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
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
}

impl Window {
    pub fn activate(&self) -> anyhow::Result<()> {
        let status = Command::new("xdotool")
            .arg("windowactivate")
            .arg("--sync")
            .arg(self.id().to_string())
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

    pub fn pid(&self) -> anyhow::Result<u32> {
        self.window.pid().map_err(Into::into)
    }

    pub fn id(&self) -> anyhow::Result<u32> {
        self.window.id().map_err(Into::into)
    }

    pub fn app_name(&self) -> anyhow::Result<String> {
        self.window.app_name().map_err(Into::into)
    }

    pub fn title(&self) -> anyhow::Result<String> {
        self.window.title().map_err(Into::into)
    }

    pub fn x(&self) -> anyhow::Result<i32> {
        self.window.x().map_err(Into::into)
    }

    pub fn y(&self) -> anyhow::Result<i32> {
        self.window.y().map_err(Into::into)
    }

    pub fn width(&self) -> anyhow::Result<u32> {
        self.window.width().map_err(Into::into)
    }

    pub fn height(&self) -> anyhow::Result<u32> {
        self.window.height().map_err(Into::into)
    }

    pub fn is_minimized(&self) -> anyhow::Result<bool> {
        self.window.is_minimized().map_err(Into::into)
    }

    pub fn is_maximized(&self) -> anyhow::Result<bool> {
        self.window.is_maximized().map_err(Into::into)
    }

    pub fn capture_image(&self) -> anyhow::Result<RgbaImage> {
        Ok(self.window.capture_image()?)
    }

    pub fn mouse_move(&self, x: i32, y: i32) -> anyhow::Result<()> {
        let global_x = self.x()? + x;
        let global_y = self.y()? + y;
        self.context.mouse_move_global(global_x, global_y)
    }
}
