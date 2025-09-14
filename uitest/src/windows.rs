#![allow(unused_variables)]

use {
    anyhow::bail,
    std::ffi::c_void,
    windows_sys::Win32::{
        Foundation::GetLastError,
        UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE},
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

    pub fn close_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        // xcap returns HWND pointer as window id.
        let ret = unsafe { PostMessageW(window.id() as *mut c_void, WM_CLOSE, 0, 0) };
        if ret == 0 {
            let error = unsafe { GetLastError() };
            bail!("failed to close window (error code: {})", error);
        }
        Ok(())
    }

    pub fn resize_window(
        &self,
        window: &crate::Window,
        width: i32,
        height: i32,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
