use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        shared_window::X11WindowType,
        types::Point,
        widgets::widget_trait::{NewWidget, WidgetInitializer},
    },
    std::fmt::Display,
    winit::window::WindowLevel,
};

pub struct Window {
    base: WidgetBaseOf<Self>,
}

impl Window {
    pub fn init(title: String) -> impl WidgetInitializer<Output = Self> {
        Initializer { title }
    }

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

    pub fn set_outer_position(&mut self, position: Point) -> &mut Self {
        self.base.window().unwrap().set_outer_position(position);
        self
    }
}

struct Initializer {
    title: String,
}

impl WidgetInitializer for Initializer {
    type Output = Window;

    fn init(self, base: WidgetBaseOf<Self::Output>) -> Self::Output {
        let mut w = Window { base };
        w.set_title(self.title);
        w
    }

    fn reinit(self, widget: &mut Self::Output) {
        widget.set_title(self.title);
    }
}

impl NewWidget for Window {
    type Arg = String;

    fn new(base: WidgetBaseOf<Self>, title: Self::Arg) -> Self {
        let mut w = Self { base };
        w.set_title(title);
        w
    }

    fn handle_declared(&mut self, title: Self::Arg) {
        self.set_title(title);
    }
}

impl Widget for Window {
    impl_widget_base!();

    fn is_window_root_type() -> bool {
        true
    }
}
