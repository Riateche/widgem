use std::{path::PathBuf, time::Duration};

use winit::event_loop::EventLoop;

use crate::{
    event_loop::{self, UserEvent},
    widgets::RootWidget,
};

pub struct AppBuilder {
    pub(crate) system_fonts: bool,
    pub(crate) custom_font_paths: Vec<PathBuf>,
    pub(crate) fixed_scale: Option<f32>,
    pub(crate) auto_repeat_delay: Option<Duration>,
    pub(crate) auto_repeat_interval: Option<Duration>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    pub fn new() -> AppBuilder {
        AppBuilder {
            system_fonts: true,
            custom_font_paths: vec![],
            fixed_scale: None,
            auto_repeat_delay: None,
            auto_repeat_interval: None,
        }
    }

    pub fn with_system_fonts(mut self, enable: bool) -> AppBuilder {
        self.system_fonts = enable;
        self
    }

    pub fn with_font(mut self, path: PathBuf) -> AppBuilder {
        self.custom_font_paths.push(path);
        self
    }

    pub fn with_scale(mut self, scale: f32) -> AppBuilder {
        self.fixed_scale = Some(scale);
        self
    }

    pub fn with_auto_repeat_delay(mut self, delay: Duration) -> AppBuilder {
        self.auto_repeat_delay = Some(delay);
        self
    }

    pub fn with_auto_repeat_interval(mut self, interval: Duration) -> AppBuilder {
        self.auto_repeat_interval = Some(interval);
        self
    }

    pub fn run(
        self,
        init: impl FnOnce(&mut RootWidget) -> anyhow::Result<()> + 'static,
    ) -> anyhow::Result<()> {
        let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
        let mut handler = event_loop::Handler::new(self, &event_loop, init);
        event_loop.run_app(&mut handler)?;
        Ok(())
    }
}

pub fn run(
    init: impl FnOnce(&mut RootWidget) -> anyhow::Result<()> + 'static,
) -> anyhow::Result<()> {
    AppBuilder::new().run(init)
}
