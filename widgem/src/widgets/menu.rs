use {
    crate::{
        callback::{Callback, Callbacks},
        event::{LayoutEvent, WindowFocusChangeEvent},
        impl_widget_base,
        items::{
            with_index::{Items, ItemsMut},
            with_key::{ItemsWithKey, ItemsWithKeyMut},
        },
        layout::{default_layout, default_size_hint_x, default_size_hint_y},
        shared_window::X11WindowType,
        system::OrWarn,
        text_editor::{Text, TextStyle},
        types::{PhysicalPixels, Point},
        widget_initializer::{self, WidgetInitializer},
        widgets::{Column, ScrollArea},
        ChildKey, Widget, WidgetBase, WidgetBaseOf, WidgetExt, WindowRectRequest,
        WindowRectResponse,
    },
    tracing::error,
    winit::window::WindowLevel,
};

pub struct Menu {
    base: WidgetBaseOf<Self>,
    window_was_focused: bool,
}

impl Menu {
    fn new(mut base: WidgetBaseOf<Self>, position: Point) -> anyhow::Result<Self> {
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
        base.set_child(SCROLL_AREA_KEY, ScrollArea::init())?
            .set_content(Column::init())?
            .add_class("menu".into());
        Ok(Menu {
            base,
            window_was_focused: false,
        })
    }

    fn set_position(&mut self, position: Point) -> &mut Self {
        if let Some(window) = self.base.window() {
            window.set_outer_position(position);
        } else {
            error!("Menu::new: missing window");
        }
        self
    }

    pub fn init(position: Point) -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_fallible_new_and_set(Self::new, Self::set_position, position)
    }

    pub fn contents(&self) -> Items<&WidgetBase> {
        let content_base = self
            .base
            .get_child::<ScrollArea>(SCROLL_AREA_KEY)
            .expect("missing scroll area child widget in menu")
            .dyn_content()
            .expect("missing scroll area content in menu")
            .base();
        Items::new(content_base)
    }

    pub fn contents_mut(&mut self) -> ItemsMut<'_> {
        let content_base = self
            .base
            .get_child_mut::<ScrollArea>(SCROLL_AREA_KEY)
            .expect("missing scroll area child widget in menu")
            .dyn_content_mut()
            .expect("missing scroll area content in menu")
            .base_mut();
        ItemsMut::new(content_base)
    }

    pub fn contents_with_key<K: Into<ChildKey>>(&self) -> ItemsWithKey<&WidgetBase, K> {
        let content_base = self
            .base
            .get_child::<ScrollArea>(SCROLL_AREA_KEY)
            .expect("missing scroll area child widget in menu")
            .dyn_content()
            .expect("missing scroll area content in menu")
            .base();
        ItemsWithKey::new(content_base)
    }

    pub fn contents_with_key_mut<K: Into<ChildKey>>(&mut self) -> ItemsWithKeyMut<'_, K> {
        let content_base = self
            .base
            .get_child_mut::<ScrollArea>(SCROLL_AREA_KEY)
            .expect("missing scroll area child widget in menu")
            .dyn_content_mut()
            .expect("missing scroll area content in menu")
            .base_mut();
        ItemsWithKeyMut::new(content_base)
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
                if let Some(window) = self.base.window_or_err().or_warn() {
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

    fn handle_window_rect_request(
        &mut self,
        _request: WindowRectRequest,
    ) -> anyhow::Result<WindowRectResponse> {
        // Ok(WindowRectResponse {
        //     position: Some(Point::new(100.ppx(), 100.ppx())),
        //     size: Some(Size::new(500.ppx(), 500.ppx())),
        // })
        Ok(WindowRectResponse {
            position: None,
            size: None,
        })
    }
}

pub struct MenuAction {
    base: WidgetBaseOf<Self>,
    text: String,
    clicked: Callbacks<()>,
}

impl MenuAction {
    fn new(base: WidgetBaseOf<Self>, text: String) -> Self {
        MenuAction {
            base,
            text,
            clicked: Default::default(),
        }
    }

    pub fn init(text: String) -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_new_and_set(Self::new, Self::set_text, text)
    }

    pub fn set_text(&mut self, text: String) -> &mut Self {
        self.text = text;
        self
    }

    pub fn on_clicked(&mut self, callback: Callback<()>) -> &mut Self {
        self.clicked.add(callback);
        self
    }
}

impl Widget for MenuAction {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let text_style = self.base.compute_style::<TextStyle>();
        self.base
            .children_mut()
            .set_next_item(Text::init(self.text.clone(), text_style))?
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
