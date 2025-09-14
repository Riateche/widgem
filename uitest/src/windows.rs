#![allow(unused_variables)]

use {
    anyhow::bail,
    std::ffi::c_void,
    windows_sys::Win32::{
        Foundation::{GetLastError, RECT},
        UI::{
            HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
            WindowsAndMessaging::{
                GetMenu, GetWindowLongW, PostMessageW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE,
                SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, WM_CLOSE,
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
        todo!()
    }

    pub fn mouse_click(&self, button: u32) -> anyhow::Result<()> {
        todo!()
    }

    pub fn mouse_down(&self, button: u32) -> anyhow::Result<()> {
        todo!()
    }

    pub fn mouse_up(&self, button: u32) -> anyhow::Result<()> {
        todo!()
    }

    pub fn key(&self, key: &str) -> anyhow::Result<()> {
        todo!()
    }

    pub fn type_text(&self, text: &str) -> anyhow::Result<()> {
        todo!()
    }

    pub fn activate_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        todo!()
    }

    pub fn mouse_move(&self, window: &crate::Window, x: u32, y: u32) -> anyhow::Result<()> {
        todo!()
    }

    pub fn minimize_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        todo!()
    }

    fn check_winapi_error(&self, success: bool) -> anyhow::Result<()> {
        if !success {
            let error = unsafe { GetLastError() };
            bail!("failed to close window (error code: {})", error);
        }
        Ok(())
    }

    pub fn close_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        // xcap returns HWND pointer as window id.
        let ret = unsafe { PostMessageW(window.id() as *mut c_void, WM_CLOSE, 0, 0) };
        self.check_winapi_error(ret != 0)
    }

    pub fn resize_window(
        &self,
        window: &crate::Window,
        width: i32,
        height: i32,
    ) -> anyhow::Result<()> {
        let hwnd = window.id() as *mut c_void;
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        self.check_winapi_error(dpi != 0)?;

        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };
        self.check_winapi_error(style != 0)?;
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
        self.check_winapi_error(ex_style != 0)?;

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
        self.check_winapi_error(ret != 0)?;

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
        self.check_winapi_error(ret != 0)
    }
}
