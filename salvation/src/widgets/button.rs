use {
    super::{image::Image, Widget, WidgetCommon, WidgetCommonTyped, WidgetExt},
    crate::{
        callback::{Callback, CallbackVec},
        draw::DrawEvent,
        event::{
            AccessibilityActionEvent, FocusReason, KeyboardInputEvent, MouseInputEvent,
            MouseMoveEvent, StyleChangeEvent,
        },
        impl_widget_common,
        layout::{
            grid::{GridAxisOptions, GridOptions},
            Alignment,
        },
        style::{button::ComputedButtonStyle, css::MyPseudoClass},
        system::{add_interval, add_timer, send_window_request, with_system},
        text_editor::Text,
        timer::TimerId,
        types::{Point, Rect},
        window::SetFocusRequest,
    },
    accesskit::{Action, Role},
    anyhow::Result,
    cosmic_text::Attrs,
    salvation_macros::impl_with,
    std::fmt::Display,
    winit::{
        event::MouseButton,
        keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    },
};

pub struct Button {
    auto_repeat: bool,
    is_mouse_leave_sensitive: bool,
    trigger_on_press: bool,
    on_triggered: CallbackVec<()>,
    is_pressed: bool,
    was_pressed_but_moved_out: bool,
    auto_repeat_delay_timer: Option<TimerId>,
    auto_repeat_interval: Option<TimerId>,
    common: WidgetCommonTyped<Self>,
}

#[impl_with]
impl Button {
    #[allow(dead_code)]
    fn image_widget(&self) -> &Image {
        self.common.get_child::<Image>(0).unwrap()
    }

    fn image_widget_mut(&mut self) -> &mut Image {
        self.common.get_child_mut::<Image>(0).unwrap()
    }

    fn text_widget(&self) -> &Text {
        self.common.get_child::<Text>(1).unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.common.get_child_mut::<Text>(1).unwrap()
    }

    pub fn set_text(&mut self, text: impl Display) -> &mut Self {
        self.text_widget_mut().set_text(text, Attrs::new());
        self.common.size_hint_changed();
        self.common.update();
        self
    }

    pub fn set_text_visible(&mut self, value: bool) -> &mut Self {
        self.text_widget_mut().set_visible(value);
        self.common.size_hint_changed();
        self.common.update();
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
        self.on_triggered.push(callback);
        self
    }

    pub fn trigger(&mut self) {
        self.on_triggered.invoke(());
    }

    fn set_pressed(&mut self, value: bool, suppress_trigger: bool) {
        if self.is_pressed == value {
            return;
        }
        self.is_pressed = value;
        if self.is_pressed {
            self.common.add_pseudo_class(MyPseudoClass::Active);
        } else {
            self.common.remove_pseudo_class(MyPseudoClass::Active);
        }
        if value {
            if self.trigger_on_press && !suppress_trigger {
                self.trigger();
            }
            let delay = with_system(|s| s.config.auto_repeat_delay);
            if self.auto_repeat {
                let id = add_timer(
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
                id.cancel();
            }
            if let Some(id) = self.auto_repeat_interval.take() {
                id.cancel();
            }
            if !self.trigger_on_press && !suppress_trigger {
                self.trigger();
            }
        }
    }

    fn start_auto_repeat(&mut self) {
        if !self.common.is_enabled() {
            return;
        }
        self.trigger();
        let interval = with_system(|s| s.config.auto_repeat_interval);
        let id = add_interval(
            interval,
            self.callback(|this, _| {
                if this.common.is_enabled() {
                    this.trigger();
                }
                Ok(())
            }),
        );
        self.auto_repeat_interval = Some(id);
    }

    fn refresh_style(&mut self) {
        let style = self.common.common_style.clone();
        self.text_widget_mut().set_font_metrics(style.font_metrics);
        self.text_widget_mut().set_text_color(style.text_color);

        let icon = self
            .common
            .specific_style::<ComputedButtonStyle>()
            .icon
            .clone();
        self.image_widget_mut().set_visible(icon.is_some());
        self.image_widget_mut().set_prescaled(true);
        self.image_widget_mut().set_pixmap(icon);

        self.common.set_grid_options(Some(GridOptions {
            x: GridAxisOptions {
                min_padding: style.min_padding_with_border.x,
                min_spacing: 0, // TODO: spacing between icon and image
                preferred_padding: style.preferred_padding_with_border.x,
                preferred_spacing: 0,
                border_collapse: 0,
                // TODO: get from style
                alignment: Alignment::Middle,
            },
            y: GridAxisOptions {
                min_padding: style.min_padding_with_border.y,
                min_spacing: 0,
                preferred_padding: style.preferred_padding_with_border.y,
                preferred_spacing: 0,
                border_collapse: 0,
                alignment: Alignment::Middle,
            },
        }));
    }
}

impl Widget for Button {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        common.set_supports_focus(true);
        common
            .add_child::<Image>()
            .set_column(0)
            .set_row(0)
            .set_visible(false);
        common.add_child::<Text>().set_column(1).set_row(0);
        let mut b = Self {
            auto_repeat: false,
            is_mouse_leave_sensitive: true,
            trigger_on_press: false,
            on_triggered: CallbackVec::new(),
            is_pressed: false,
            was_pressed_but_moved_out: false,
            common,
            auto_repeat_delay_timer: None,
            auto_repeat_interval: None,
        };
        b.refresh_style();
        b
    }

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        // TODO: too expensive! refactor styles
        self.refresh_style();
        let size = self.common.size_or_err()?;
        let style = &self.common.common_style;

        event.stroke_and_fill_rounded_rect(
            Rect {
                top_left: Point::default(),
                size,
            },
            &style.border,
            style.background.as_ref(),
        );

        Ok(())
    }

    fn handle_mouse_move(&mut self, event: MouseMoveEvent) -> Result<bool> {
        let rect = self.common.rect_in_self_or_err()?;
        if rect.contains(event.pos) {
            if self.was_pressed_but_moved_out {
                self.was_pressed_but_moved_out = true;
                self.set_pressed(true, true);
                self.common.update();
            }
        } else {
            if self.is_pressed && self.is_mouse_leave_sensitive {
                self.was_pressed_but_moved_out = true;
                self.set_pressed(false, true);
                self.common.update();
            }
        }
        Ok(true)
    }

    fn handle_mouse_input(&mut self, event: MouseInputEvent) -> Result<bool> {
        if !self.common.is_enabled() {
            return Ok(true);
        }
        if event.button == MouseButton::Left {
            if event.state.is_pressed() {
                self.set_pressed(true, false);
                if !self.common.is_focused() {
                    let window = self.common.window_or_err()?;
                    if self.common.is_focusable() {
                        send_window_request(
                            window.id(),
                            SetFocusRequest {
                                widget_id: self.common.id().into(),
                                reason: FocusReason::Mouse,
                            },
                        );
                    }
                }
            } else {
                self.was_pressed_but_moved_out = false;
                self.set_pressed(false, false);
            }
            self.common.update();
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
                send_window_request(
                    self.common.window_or_err()?.id(),
                    SetFocusRequest {
                        widget_id: self.common.id().into(),
                        // TODO: separate reason?
                        reason: FocusReason::Mouse,
                    },
                );
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
        self.refresh_style();
        self.common.size_hint_changed();
        self.common.update();
        Ok(())
    }
}
