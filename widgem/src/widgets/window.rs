use {
    super::{Widget, WidgetBaseOf},
    crate::{
        event_loop::with_active_event_loop, impl_widget_base, shared_window::X11WindowType,
        system::with_system, widgets::widget_trait::NewWidget, WidgetExt,
    },
    log::warn,
    std::fmt::Display,
    winit::window::WindowLevel,
};

fn default_scale() -> f32 {
    if let Some(scale) = with_system(|system| system.config.fixed_scale) {
        return scale;
    }
    with_active_event_loop(|event_loop| {
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());
        if let Some(monitor) = monitor {
            monitor.scale_factor() as f32
        } else {
            warn!("unable to find any monitors");
            1.0
        }
    })
}

pub struct Window {
    base: WidgetBaseOf<Self>,
}

impl Window {
    pub fn set_title(&mut self, title: impl Display) -> &mut Self {
        self.base.window().unwrap().set_title(title);
        self
    }

    pub fn set_decorations(&mut self, value: bool) -> &mut Self {
        self.base.window().unwrap().set_decorations(value);
        self
    }

    pub fn set_window_level(&mut self, value: WindowLevel) -> &mut Self {
        self.base.window().unwrap().set_window_level(value);
        self
    }

    pub fn set_x11_window_type(&mut self, value: Vec<X11WindowType>) -> &mut Self {
        self.base.window().unwrap().set_x11_window_type(value);
        self
    }

    pub fn set_skip_windows_taskbar(&mut self, value: bool) -> &mut Self {
        self.base.window().unwrap().set_skip_windows_taskbar(value);
        self
    }
}

impl NewWidget for Window {
    type Arg = String;

    fn new(base: WidgetBaseOf<Self>, arg: Self::Arg) -> Self {
        let mut w = Self { base };
        w.set_title(arg);
        w.set_scale(Some(default_scale()));
        w
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_title(arg);
    }
}

impl Widget for Window {
    impl_widget_base!();

    fn is_window_root_type() -> bool {
        true
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.base.window().unwrap().deregister();
    }
}
