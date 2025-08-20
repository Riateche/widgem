use {
    super::{Widget, WidgetBaseOf},
    crate::{
        callback::{Callback, Callbacks},
        event::{LayoutEvent, WindowFocusChangeEvent},
        impl_widget_base,
        layout::{default_layout, default_size_hint_x, default_size_hint_y},
        shared_window::X11WindowType,
        system::ReportError,
        text_editor::{Text, TextStyle},
        types::{PhysicalPixels, Point},
        widgets::{widget_trait::NewWidget, Column, ScrollArea},
        WidgetExt,
    },
    tracing::error,
    winit::window::WindowLevel,
};

pub struct Menu {
    base: WidgetBaseOf<Self>,
    window_was_focused: bool,
}

impl Menu {
    pub fn content(&self) -> &dyn Widget {
        self.base
            .get_child::<ScrollArea>(SCROLL_AREA_KEY)
            .expect("missing scroll area child widget in menu")
            .dyn_content()
            .expect("missing scroll area content in menu")
    }

    pub fn content_mut(&mut self) -> &mut dyn Widget {
        self.base
            .get_child_mut::<ScrollArea>(SCROLL_AREA_KEY)
            .expect("missing scroll area child widget in menu")
            .dyn_content_mut()
            .expect("missing scroll area content in menu")
    }
    // pub fn delete_on_close(&self) -> bool {
    //     self.delete_on_close
    // }

    // pub fn set_delete_on_close(&mut self, delete_on_close: bool) -> &mut Self {
    //     if let Some(window) = self.base.window_or_err().or_report_err() {
    //         window.set
    //     }
    //     self
    // }
}

const SCROLL_AREA_KEY: u64 = 0;

impl NewWidget for Menu {
    type Arg = Point;

    fn new(mut base: WidgetBaseOf<Self>, position: Self::Arg) -> Self {
        if let Some(window) = base.window() {
            window.set_title("Menu"); // TODO: translations
            window.set_decorations(false);
            window.set_has_macos_shadow(false);
            window.set_resizable(false);
            window.set_window_level(WindowLevel::AlwaysOnTop);
            window.set_x11_window_type(vec![X11WindowType::PopupMenu]);
            window.set_skip_windows_taskbar(true);
            window.set_outer_position(position);
        } else {
            error!("Menu::new: missing window");
        }
        base.add_child_with_key::<ScrollArea>(SCROLL_AREA_KEY, ())
            .set_content::<Column>(())
            .add_class("menu".into());
        Self {
            base,
            window_was_focused: false,
            //delete_on_close: false,
        }
    }

    fn handle_declared(&mut self, position: Self::Arg) {
        if let Some(window) = self.base.window() {
            window.set_outer_position(position);
        } else {
            error!("Menu::new: missing window");
        }
    }
}

impl Widget for Menu {
    impl_widget_base!();

    fn is_window_root_type() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn handle_window_focus_change(&mut self, event: WindowFocusChangeEvent) -> anyhow::Result<()> {
        if event.is_window_focused() {
            self.window_was_focused = true;
        } else {
            if self.window_was_focused {
                //self.set_visible(false);
                if let Some(window) = self.base.window_or_err().or_report_err() {
                    window.close();
                }
            }
        }
        Ok(())
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> anyhow::Result<()> {
        default_layout(self);
        Ok(())
    }
}

pub struct MenuItem {
    base: WidgetBaseOf<Self>,
    text: String,
    clicked: Callbacks<()>,
}

impl MenuItem {
    pub fn set_text(&mut self, text: &str) -> &mut Self {
        self.text = text.into();
        self
    }

    pub fn on_clicked(&mut self, callback: Callback<()>) -> &mut Self {
        self.clicked.add(callback);
        self
    }
}

impl NewWidget for MenuItem {
    type Arg = String;

    fn new(base: WidgetBaseOf<Self>, text: Self::Arg) -> Self {
        Self {
            base,
            text,
            clicked: Default::default(),
        }
    }

    fn handle_declared(&mut self, text: Self::Arg) {
        self.set_text(&text);
    }
}

impl Widget for MenuItem {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let text_style = self.base.compute_style::<TextStyle>();
        self.base
            .declare_child::<Text>((self.text.clone(), text_style))
            .set_multiline(false);
        Ok(())
    }

    // Menu items are not really expanding.
    // However, the menu's OS window is sometimes slightly larger than requested.
    // In that case we want menu items to take all available space.
    fn handle_size_hint_x_request(
        &self,
        size_y: Option<PhysicalPixels>,
    ) -> anyhow::Result<crate::layout::SizeHint> {
        let mut size_hint = default_size_hint_x(self, size_y);
        size_hint.set_fixed(false);
        Ok(size_hint)
    }

    fn handle_size_hint_y_request(
        &self,
        size_x: crate::types::PhysicalPixels,
    ) -> anyhow::Result<crate::layout::SizeHint> {
        let mut size_hint = default_size_hint_y(self, size_x);
        size_hint.set_fixed(false);
        Ok(size_hint)
    }
}
