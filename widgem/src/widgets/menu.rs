use {
    crate::{
        callback::{Callback, Callbacks},
        event::{
            KeyboardInputEvent, LayoutEvent, MouseLeaveEvent, MouseMoveEvent,
            WindowFocusChangeEvent,
        },
        impl_widget_base,
        items::{
            with_index::{Items, ItemsMut},
            with_key::{ItemsWithKey, ItemsWithKeyMut},
        },
        layout::{default_layout, default_size_hint_x, default_size_hint_y},
        shared_window::X11WindowType,
        style::css::PseudoClass,
        system::OrWarn,
        text_editor::{Text, TextStyle},
        types::{PhysicalPixels, Point},
        widget_initializer::{self, WidgetInitializer},
        widgets::{Column, ScrollArea},
        ChildKey, Widget, WidgetBase, WidgetBaseOf, WidgetExt, WindowRectRequest,
        WindowRectResponse,
    },
    tracing::error,
    winit::{
        event::ElementState,
        keyboard::{Key, NamedKey},
        window::WindowLevel,
    },
};

pub struct Menu {
    base: WidgetBaseOf<Self>,
    window_was_focused: bool,
    current_key: Option<ChildKey>,
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
        base.set_supports_focus(true);
        Ok(Menu {
            base,
            window_was_focused: false,
            current_key: None,
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

    fn contents_base(&self) -> &WidgetBase {
        self.base
            .get_child::<ScrollArea>(SCROLL_AREA_KEY)
            .expect("missing scroll area child widget in menu")
            .dyn_content()
            .expect("missing scroll area content in menu")
            .base()
    }

    fn contents_base_mut(&mut self) -> &mut WidgetBase {
        self.base
            .get_child_mut::<ScrollArea>(SCROLL_AREA_KEY)
            .expect("missing scroll area child widget in menu")
            .dyn_content_mut()
            .expect("missing scroll area content in menu")
            .base_mut()
    }

    pub fn contents_with_key<K: Into<ChildKey>>(&self) -> ItemsWithKey<&WidgetBase, K> {
        ItemsWithKey::new(self.contents_base())
    }

    pub fn contents_with_key_mut<K: Into<ChildKey>>(&mut self) -> ItemsWithKeyMut<'_, K> {
        ItemsWithKeyMut::new(self.contents_base_mut())
    }

    // pub fn delete_on_close(&self) -> bool {
    //     self.delete_on_close
    // }

    // pub fn set_delete_on_close(&mut self, delete_on_close: bool) -> &mut Self {
    //     if let Some(window) = self.base.window_or_err().or_warn() {
    //         window.set
    //     }
    //     self
    // }

    fn checked_current_key(&self) -> Option<ChildKey> {
        let contents_base = self.contents_base();
        let Some(current_key) = &self.current_key else {
            return None;
        };
        let Ok(item) = contents_base.get_dyn_child(current_key.clone()) else {
            return None;
        };
        if item.base().has_pseudo_class(PseudoClass::Current) {
            Some(current_key.clone())
        } else {
            None
        }
    }

    fn arrow_down(&mut self) {
        if let Some(old_key) = self.checked_current_key() {
            let contents_base = self.contents_base_mut();
            let Some(new_key) = contents_base.child_key_after(&old_key).cloned() else {
                return;
            };
            let Ok(new_child) = contents_base.get_dyn_child_mut(new_key.clone()) else {
                error!("child_key_after returned invalid key");
                return;
            };
            new_child.base_mut().add_pseudo_class(PseudoClass::Current);
            self.current_key = Some(new_key);
            let contents_base = self.contents_base_mut();
            let Ok(old_child) = contents_base.get_dyn_child_mut(old_key) else {
                return;
            };
            old_child
                .base_mut()
                .remove_pseudo_class(PseudoClass::Current);
        } else {
            let contents_base = self.contents_base_mut();
            let Some(new_key) = contents_base.child_keys().next().cloned() else {
                return;
            };
            let Ok(new_child) = contents_base.get_dyn_child_mut(new_key.clone()) else {
                error!("child_key_after returned invalid key");
                return;
            };
            new_child.base_mut().add_pseudo_class(PseudoClass::Current);
            self.current_key = Some(new_key);
        }
    }

    fn arrow_up(&mut self) {
        if let Some(old_key) = self.checked_current_key() {
            let contents_base = self.contents_base_mut();
            let Some(new_key) = contents_base.child_key_before(&old_key).cloned() else {
                return;
            };
            let Ok(new_child) = contents_base.get_dyn_child_mut(new_key.clone()) else {
                error!("child_key_after returned invalid key");
                return;
            };
            new_child.base_mut().add_pseudo_class(PseudoClass::Current);
            self.current_key = Some(new_key);
            let contents_base = self.contents_base_mut();
            let Ok(old_child) = contents_base.get_dyn_child_mut(old_key) else {
                return;
            };
            old_child
                .base_mut()
                .remove_pseudo_class(PseudoClass::Current);
        } else {
            let contents_base = self.contents_base_mut();
            let Some(new_key) = contents_base.child_keys().next_back().cloned() else {
                return;
            };
            let Ok(new_child) = contents_base.get_dyn_child_mut(new_key.clone()) else {
                error!("child_key_after returned invalid key");
                return;
            };
            new_child.base_mut().add_pseudo_class(PseudoClass::Current);
            self.current_key = Some(new_key);
        }
    }
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

    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> anyhow::Result<bool> {
        if event.info().state != ElementState::Pressed {
            return Ok(false);
        }
        match event.info().logical_key {
            Key::Named(key) => match key {
                NamedKey::Enter => {}
                NamedKey::Space => {}
                NamedKey::ArrowDown => {
                    self.arrow_down();
                }
                NamedKey::ArrowLeft => {}
                NamedKey::ArrowRight => {}
                NamedKey::ArrowUp => {
                    self.arrow_up();
                }
                NamedKey::Escape => {
                    if let Some(window) = self.base.window_or_err().or_warn() {
                        window.close();
                    }
                }
                _ => {}
            },
            Key::Character(_) => {
                // TODO: check accelerators?
            }
            Key::Unidentified(_) | Key::Dead(_) => {}
        }
        Ok(false)
    }

    fn handle_mouse_move(&mut self, event: MouseMoveEvent) -> anyhow::Result<bool> {
        let pos_in_window = event.pos_in_window();
        let old_key = self.current_key.clone();
        let contents_base = self.contents_base_mut();
        let mut new_key = None;

        for (key, child) in contents_base.children_with_keys_mut() {
            let Some(child_geometry) = child.base().geometry() else {
                continue;
            };
            if child_geometry.rect_in_window().contains(pos_in_window) {
                child.base_mut().add_pseudo_class(PseudoClass::Current);
                new_key = Some(key.clone());
                break;
            }
        }

        let Some(new_key) = new_key else {
            return Ok(false);
        };
        if self.current_key.as_ref() == Some(&new_key) {
            return Ok(true);
        }
        self.current_key = Some(new_key);

        if let Some(old_key) = old_key {
            let contents_base = self.contents_base_mut();
            if let Ok(old_child) = contents_base.get_dyn_child_mut(old_key) {
                old_child
                    .base_mut()
                    .remove_pseudo_class(PseudoClass::Current);
            }
        }

        Ok(true)
    }

    fn handle_mouse_leave(&mut self, _event: MouseLeaveEvent) -> anyhow::Result<()> {
        if let Some(old_key) = self.current_key.clone() {
            let contents_base = self.contents_base_mut();
            if let Ok(old_child) = contents_base.get_dyn_child_mut(old_key) {
                old_child
                    .base_mut()
                    .remove_pseudo_class(PseudoClass::Current);
            }
        }
        self.current_key = None;
        Ok(())
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
