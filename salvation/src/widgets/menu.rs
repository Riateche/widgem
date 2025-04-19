use {
    super::{label::Label, Widget, WidgetCommon, WidgetCommonTyped},
    crate::impl_widget_common,
    winit::window::{WindowAttributes, WindowLevel},
};

pub struct Menu {
    common: WidgetCommon,
}

impl Menu {}

impl Widget for Menu {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
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
        common
            .add_child_window::<Label>(0, attrs)
            .set_text("menu content 1\nmenu content 2\nmenu content 3");
        Self {
            common: common.into(),
        }
    }
}
