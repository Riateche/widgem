use {
    super::{label::Label, Widget, WidgetCommon, WidgetExt},
    crate::{impl_widget_common, layout::LayoutItemOptions},
    winit::window::{WindowAttributes, WindowLevel},
};

pub struct Menu {
    common: WidgetCommon,
}

impl Menu {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new::<Self>();
        #[allow(unused_mut)]
        let mut attrs = WindowAttributes::default()
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop);

        #[cfg(all(unix, not(target_vendor = "apple")))]
        {
            use winit::platform::x11::{WindowAttributesExtX11, WindowType};

            attrs = attrs.with_x11_window_type(vec![WindowType::Menu]);
        }
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowAttributesExtWindows;

            attrs = attrs.with_skip_taskbar(true);
        }
        let content =
            Label::new("menu content 1\nmenu content 2\nmenu content 3").with_window(attrs);
        common.add_child(content.boxed(), LayoutItemOptions::default());
        Self {
            common: common.into(),
        }
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Menu {
    impl_widget_common!();
}
