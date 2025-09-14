#![allow(unused_variables)]

pub struct Context {}

impl Context {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn active_window_id(&self) -> anyhow::Result<u32> {
        todo!()
    }

    pub fn activate_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        todo!()
    }

    pub fn minimize_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        todo!()
    }

    pub fn close_window(&self, window: &crate::Window) -> anyhow::Result<()> {
        todo!()
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
