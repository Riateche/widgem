use xcap::{image::RgbaImage, XCapResult};

use crate::Context;

#[derive(Clone)]
pub struct Window {
    id: u32,
    pid: u32,
    inner: xcap::Window,
    context: Context,
}

impl Window {
    pub(crate) fn new(context: Context, inner: xcap::Window) -> anyhow::Result<Self> {
        Ok(Self {
            id: inner.id()?,
            pid: inner.pid()?,
            inner,
            context,
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
        self.context.0.imp.activate_window(self)
    }

    pub fn mouse_move(&self, x: i32, y: i32) -> anyhow::Result<()> {
        let global_x = self.x()? + x;
        let global_y = self.y()? + y;
        self.context.mouse_move_global(global_x, global_y)
    }

    pub fn minimize(&self) -> anyhow::Result<()> {
        self.context.0.imp.minimize_window(self)
    }

    pub fn close(&self) -> anyhow::Result<()> {
        self.context.0.imp.close_window(self)
    }

    pub fn resize(&self, width: i32, height: i32) -> anyhow::Result<()> {
        self.context.0.imp.resize_window(self, width, height)
    }
}
