use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        items::{
            with_index::{Items, ItemsMut},
            with_key::{ItemsWithKey, ItemsWithKeyMut},
        },
        shared_window::X11WindowType,
        types::Point,
        widget_initializer::WidgetInitializer,
        ChildKey, WidgetBase,
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

    pub fn set_resizable(&mut self, value: bool) -> &mut Self {
        self.base.window().unwrap().set_resizable(value);
        self
    }

    pub fn is_resizable(&self) -> bool {
        self.base.window().unwrap().is_resizable()
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

    pub fn set_main_content<WI: WidgetInitializer>(&mut self, initializer: WI) -> &mut WI::Output {
        self.base.set_main_child(initializer)
    }

    pub fn contents(&self) -> Items<&WidgetBase> {
        Items::new(&self.base)
    }

    pub fn contents_mut(&mut self) -> ItemsMut<'_> {
        ItemsMut::new(&mut self.base)
    }

    pub fn contents_with_key<K: Into<ChildKey>>(&self) -> ItemsWithKey<&WidgetBase, K> {
        ItemsWithKey::new(&self.base)
    }

    pub fn contents_with_key_mut<K: Into<ChildKey>>(&mut self) -> ItemsWithKeyMut<'_, K> {
        ItemsWithKeyMut::new(&mut self.base)
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

impl Widget for Window {
    impl_widget_base!();

    fn is_window_root_type() -> bool {
        true
    }
}
