use {
    anyhow::bail,
    image::RgbaImage,
    std::ffi::c_void,
    windows_sys::Win32::{
        Foundation::{GetLastError, RECT},
        UI::{
            HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
            Input::KeyboardAndMouse::GetActiveWindow,
            WindowsAndMessaging::{
                GetMenu, GetWindowLongW, PostMessageW, SetForegroundWindow, SetWindowPos,
                GWL_EXSTYLE, GWL_STYLE, SC_MINIMIZE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
                WM_CLOSE, WM_SYSCOMMAND,
            },
        },
    },
};

pub struct Context {}

impl Context {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn active_window_id(&self) -> anyhow::Result<u32> {
        let ret = unsafe { GetActiveWindow() };
        check_winapi_error(!ret.is_null())?;
        Ok(ret as u32)
    }
}

fn check_winapi_error(success: bool) -> anyhow::Result<()> {
    if !success {
        let error = unsafe { GetLastError() };
        bail!("failed to close window (error code: {})", error);
    }
    Ok(())
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

#[derive(Clone)]
pub struct Window {
    window: xcap::Window,
    context: crate::Context,
}

impl Window {
    pub fn activate(&self) -> anyhow::Result<()> {
        let ret = unsafe { SetForegroundWindow(self.id() as *mut c_void) };
        check_winapi_error(ret != 0)
    }

    pub fn minimize(&self) -> anyhow::Result<()> {
        let ret = unsafe {
            PostMessageW(
                self.id() as *mut c_void,
                WM_SYSCOMMAND,
                SC_MINIMIZE as usize,
                0,
            )
        };
        check_winapi_error(ret != 0)
    }

    pub fn close(&self) -> anyhow::Result<()> {
        // xcap returns HWND pointer as window id.
        let ret = unsafe { PostMessageW(self.id() as *mut c_void, WM_CLOSE, 0, 0) };
        check_winapi_error(ret != 0)
    }

    pub fn resize(&self, width: i32, height: i32) -> anyhow::Result<()> {
        let hwnd = self.id() as *mut c_void;
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        check_winapi_error(dpi != 0)?;

        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };
        check_winapi_error(style != 0)?;
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
        check_winapi_error(ex_style != 0)?;

        let menu = unsafe { GetMenu(hwnd) };

        let mut in_out_rect = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        let ret = unsafe {
            AdjustWindowRectExForDpi(
                &mut in_out_rect,
                style as u32,
                (!menu.is_null()).into(),
                ex_style as u32,
                dpi,
            )
        };
        check_winapi_error(ret != 0)?;

        let ret = unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                in_out_rect.right - in_out_rect.left,
                in_out_rect.bottom - in_out_rect.top,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOOWNERZORDER,
            )
        };
        check_winapi_error(ret != 0)
    }

    pub fn pid(&self) -> u32 {
        self.window.pid().unwrap() // TODO: Result in interface
    }

    /// The window id
    pub fn id(&self) -> u32 {
        self.window.id().unwrap() // TODO: Result in interface
    }
    /// The window app name
    pub fn app_name(&self) -> anyhow::Result<String> {
        self.window.app_name().map_err(Into::into)
    }
    /// The window title
    pub fn title(&self) -> anyhow::Result<String> {
        self.window.title().map_err(Into::into)
    }
    /// The window x coordinate.
    pub fn x(&self) -> anyhow::Result<i32> {
        self.window.x().map_err(Into::into)
    }
    /// The window x coordinate.
    pub fn y(&self) -> anyhow::Result<i32> {
        self.window.y().map_err(Into::into)
    }
    /// The window pixel width.
    pub fn width(&self) -> anyhow::Result<u32> {
        self.window.width().map_err(Into::into)
    }
    /// The window pixel height.
    pub fn height(&self) -> anyhow::Result<u32> {
        self.window.height().map_err(Into::into)
    }
    /// The window is minimized.
    pub fn is_minimized(&self) -> anyhow::Result<bool> {
        self.window.is_minimized().map_err(Into::into)
    }
    /// The window is maximized.
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
