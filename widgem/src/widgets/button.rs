use {
    super::{image::Image, Widget, WidgetBaseOf, WidgetExt},
    crate::{
        callback::{Callback, Callbacks},
        event::{
            AccessibilityActionEvent, FocusReason, KeyboardInputEvent, MouseInputEvent,
            MouseMoveEvent, StyleChangeEvent,
        },
        impl_widget_base,
        layout::Layout,
        style::{
            common::ComputedElementStyle,
            css::{convert_content_url, convert_zoom, PseudoClass, StyleSelector},
            Styles,
        },
        text_editor::Text,
        timer::TimerId,
        widgets::widget_trait::{NewWidget, WidgetInitializer},
        Pixmap,
    },
    accesskit::{Action, Role},
    anyhow::Result,
    cosmic_text::Attrs,
    std::{fmt::Display, rc::Rc},
    tracing::warn,
    widgem_macros::impl_with,
    winit::{
        event::MouseButton,
        keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    },
};

pub struct Button {
    auto_repeat: bool,
    is_mouse_leave_sensitive: bool,
    trigger_on_press: bool,
    on_triggered: Callbacks<()>,
    is_pressed: bool,
    was_pressed_but_moved_out: bool,
    auto_repeat_delay_timer: Option<TimerId>,
    auto_repeat_interval: Option<TimerId>,
    base: WidgetBaseOf<Self>,
    style: Rc<ComputedButtonStyle>,
}

#[impl_with]
impl Button {
    pub fn init(text: String) -> impl WidgetInitializer<Output = Self> {
        Initializer { text }
    }

    #[allow(dead_code)]
    fn image_widget(&self) -> &Image {
        self.base.get_child::<Image>(0).unwrap()
    }

    fn image_widget_mut(&mut self) -> &mut Image {
        self.base.get_child_mut::<Image>(0).unwrap()
    }

    fn text_widget(&self) -> &Text {
        self.base.get_child::<Text>(1).unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.base.get_child_mut::<Text>(1).unwrap()
    }

    pub fn set_text(&mut self, text: impl Display) -> &mut Self {
        self.text_widget_mut().set_text(text, Attrs::new());
        self.base.size_hint_changed();
        self.base.update();
        self
    }

    pub fn set_text_visible(&mut self, value: bool) -> &mut Self {
        self.text_widget_mut().set_visible(value);
        self.base.size_hint_changed();
        self.base.update();
        self
    }

    pub fn set_auto_repeat(&mut self, value: bool) -> &mut Self {
        self.auto_repeat = value;
        self
    }

    pub fn set_mouse_leave_sensitive(&mut self, value: bool) -> &mut Self {
        self.is_mouse_leave_sensitive = value;
        self
    }

    pub fn set_trigger_on_press(&mut self, value: bool) -> &mut Self {
        self.trigger_on_press = value;
        self
    }

    // TODO: set_icon should preferably work with SVG icons
    // pub fn set_icon(&mut self, icon: Option<Rc<Pixmap>>) {
    //     self.icon = icon;
    //     self.common.size_hint_changed();
    //     self.common.update();
    // }

    pub fn on_triggered(&mut self, callback: Callback<()>) -> &mut Self {
        self.on_triggered.add(callback);
        self
    }

    pub fn trigger(&mut self) {
        self.on_triggered.invoke((), false);
    }

    fn set_pressed(&mut self, value: bool, suppress_trigger: bool) {
        if self.is_pressed == value {
            return;
        }
        self.is_pressed = value;
        self.set_pseudo_class(PseudoClass::Active, self.is_pressed);
        if value {
            if self.trigger_on_press && !suppress_trigger {
                self.trigger();
            }
            let delay = self.base.app().config().auto_repeat_delay;
            if self.auto_repeat {
                let id = self.base.app().add_timer(
                    delay,
                    self.callback(|this, _| {
                        this.start_auto_repeat();
                        Ok(())
                    }),
                );
                self.auto_repeat_delay_timer = Some(id);
            }
        } else {
            if let Some(id) = self.auto_repeat_delay_timer.take() {
                self.base.app().cancel_timer(id);
            }
            if let Some(id) = self.auto_repeat_interval.take() {
                self.base.app().cancel_timer(id);
            }
            if !self.trigger_on_press && !suppress_trigger {
                self.trigger();
            }
        }
    }

    fn start_auto_repeat(&mut self) {
        if !self.base.is_enabled() {
            return;
        }
        self.trigger();
        let interval = self.base.app().config().auto_repeat_interval;
        let id = self.base.app().add_interval(
            interval,
            self.callback(|this, _| {
                if this.base.is_enabled() {
                    this.trigger();
                }
                Ok(())
            }),
        );
        self.auto_repeat_interval = Some(id);
    }

    fn refresh_style(&mut self) {
        self.style = self.base.compute_style();
        let icon = self.style.icon.clone();
        self.image_widget_mut().set_visible(icon.is_some());
        self.image_widget_mut().set_prescaled(true);
        self.image_widget_mut().set_pixmap(icon);
    }
}

struct Initializer {
    text: String,
}

impl WidgetInitializer for Initializer {
    type Output = Button;

    fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
        base.set_supports_focus(true);
        base.set_layout(Layout::HorizontalFirst);
        base.add_child(Image::init(None)).set_visible(false);
        let id = base.id().raw();
        let text_style = base.compute_style();
        base.add_child(Text::init(self.text, text_style))
            .set_host_id(id);
        let mut b = Button {
            style: base.compute_style(),
            auto_repeat: false,
            is_mouse_leave_sensitive: true,
            trigger_on_press: false,
            on_triggered: Callbacks::default(),
            is_pressed: false,
            was_pressed_but_moved_out: false,
            base,
            auto_repeat_delay_timer: None,
            auto_repeat_interval: None,
        };
        // TODO: remove and use declare_children
        b.refresh_style();
        b
    }

    fn reinit(self, widget: &mut Self::Output) {
        widget.set_text(self.text);
    }
}

impl NewWidget for Button {
    type Arg = String;

    fn new(mut base: WidgetBaseOf<Self>, text: Self::Arg) -> Self {
        base.set_supports_focus(true);
        base.set_layout(Layout::HorizontalFirst);
        base.add_child(Image::init(None)).set_visible(false);
        let id = base.id().raw();
        let text_style = base.compute_style();
        base.add_child(Text::init(text, text_style)).set_host_id(id);
        let mut b = Self {
            style: base.compute_style(),
            auto_repeat: false,
            is_mouse_leave_sensitive: true,
            trigger_on_press: false,
            on_triggered: Callbacks::default(),
            is_pressed: false,
            was_pressed_but_moved_out: false,
            base,
            auto_repeat_delay_timer: None,
            auto_repeat_interval: None,
        };
        // TODO: remove and use declare_children
        b.refresh_style();
        b
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_text(arg);
    }
}

impl Widget for Button {
    impl_widget_base!();

    fn handle_mouse_move(&mut self, event: MouseMoveEvent) -> Result<bool> {
        let rect = self.base.rect_in_self_or_err()?;
        if rect.contains(event.pos) {
            if self.was_pressed_but_moved_out {
                self.was_pressed_but_moved_out = true;
                self.set_pressed(true, true);
                self.base.update();
            }
        } else {
            if self.is_pressed && self.is_mouse_leave_sensitive {
                self.was_pressed_but_moved_out = true;
                self.set_pressed(false, true);
                self.base.update();
            }
        }
        Ok(true)
    }

    fn handle_mouse_input(&mut self, event: MouseInputEvent) -> Result<bool> {
        if !self.base.is_enabled() {
            return Ok(true);
        }
        if event.button == MouseButton::Left {
            if event.state.is_pressed() {
                self.set_pressed(true, false);
                if !self.base.is_focused() {
                    if self.base.is_focusable() {
                        self.base.set_focus(FocusReason::Mouse);
                    }
                }
            } else {
                self.was_pressed_but_moved_out = false;
                self.set_pressed(false, false);
            }
            self.base.update();
        }
        Ok(true)
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        if event.info.physical_key == PhysicalKey::Code(KeyCode::Space)
            || event.info.logical_key == Key::Named(NamedKey::Space)
        {
            self.set_pressed(event.info.state.is_pressed(), false);
            return Ok(true);
        }
        if event.info.physical_key == PhysicalKey::Code(KeyCode::Enter)
            || event.info.physical_key == PhysicalKey::Code(KeyCode::NumpadEnter)
            || event.info.logical_key == Key::Named(NamedKey::Enter)
        {
            self.trigger();
            return Ok(true);
        }
        Ok(false)
    }

    fn handle_accessibility_action(&mut self, event: AccessibilityActionEvent) -> Result<()> {
        match event.action {
            Action::Click => self.trigger(),
            Action::Focus => {
                self.base.set_focus(FocusReason::Mouse);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_accessibility_node_request(&mut self) -> Result<Option<accesskit::Node>> {
        let mut node = accesskit::Node::new(Role::Button);
        node.set_label(self.text_widget().text().as_str());
        node.add_action(Action::Click);
        node.add_action(Action::Focus);
        Ok(Some(node))
    }

    fn handle_style_change(&mut self, _event: StyleChangeEvent) -> Result<()> {
        let text_style = self.base.compute_style();
        self.text_widget_mut().set_text_style(text_style);
        self.refresh_style();
        self.base.size_hint_changed();
        self.base.update();
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct ComputedButtonStyle {
    pub icon: Option<Pixmap>,
}

impl ComputedElementStyle for ComputedButtonStyle {
    fn new(style: &Styles, element: &StyleSelector, scale: f32) -> ComputedButtonStyle {
        let properties = style.find_rules(|s| element.matches(s));

        let scale = scale * convert_zoom(&properties);
        let mut icon = None;
        if let Some(url) = convert_content_url(&properties) {
            //println!("icon url: {url:?}");
            match style.load_pixmap(&url, scale) {
                Ok(pixmap) => icon = Some(pixmap),
                Err(err) => warn!("failed to load icon: {err:?}"),
            }
        }
        Self { icon }
    }
}
