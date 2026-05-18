use {
    crate::{
        clean::{app_handler::AppHandler, BoxWidget},
        event_loop::UserEvent,
    },
    std::{path::PathBuf, time::Duration},
    strict_num::FiniteF32,
    tracing::warn,
    winit::event_loop::{ActiveEventLoop, EventLoop},
};

pub struct App {
    system_fonts: bool,
    custom_font_paths: Vec<PathBuf>,
    fixed_scale: Option<FiniteF32>,
    auto_repeat_delay: Option<Duration>,
    auto_repeat_interval: Option<Duration>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> App {
        App {
            system_fonts: true,
            custom_font_paths: vec![],
            fixed_scale: None,
            auto_repeat_delay: None,
            auto_repeat_interval: None,
        }
    }

    pub fn system_fonts(mut self, enable: bool) -> App {
        self.system_fonts = enable;
        self
    }

    pub fn add_font(mut self, path: PathBuf) -> App {
        self.custom_font_paths.push(path);
        self
    }

    pub fn scale(mut self, scale: FiniteF32) -> App {
        self.fixed_scale = Some(scale);
        self
    }

    pub fn auto_repeat_delay(mut self, delay: Duration) -> App {
        self.auto_repeat_delay = Some(delay);
        self
    }

    pub fn auto_repeat_interval(mut self, interval: Duration) -> App {
        self.auto_repeat_interval = Some(interval);
        self
    }

    pub fn run(self, root: BoxWidget) -> anyhow::Result<()> {
        let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
        let mut handler = AppHandler::new(self, root);
        event_loop.run_app(&mut handler)?;
        Ok(())
    }

    pub(crate) fn default_scale(&self, event_loop: ActiveEventLoop) -> FiniteF32 {
        if let Some(scale) = self.fixed_scale {
            return scale;
        }
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());
        if let Some(monitor) = monitor {
            FiniteF32::new(monitor.scale_factor() as f32).unwrap_or_else(|| {
                warn!("monitor scale is not finite");
                FiniteF32::new(1.0).unwrap()
            })
        } else {
            warn!("unable to find any monitors");
            FiniteF32::new(1.0).unwrap()
        }
    }
}

pub fn run(root: BoxWidget) -> anyhow::Result<()> {
    App::new().run(root)
}
