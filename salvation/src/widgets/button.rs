use std::{cmp::max, fmt::Display, rc::Rc};

use accesskit::{Action, DefaultActionVerb, NodeBuilder, Role};
use anyhow::Result;
use log::warn;
use salvation_cosmic_text::Attrs;
use salvation_macros::impl_with;
use tiny_skia::Pixmap;
use winit::{
    event::MouseButton,
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
};

use crate::{
    callback::{Callback, CallbackVec},
    draw::DrawEvent,
    event::{
        AccessibleActionEvent, FocusReason, KeyboardInputEvent, MouseInputEvent, MouseMoveEvent,
        WidgetScopeChangeEvent,
    },
    impl_widget_common,
    layout::SizeHintMode,
    style::button::{ButtonState, ComputedStyle, ComputedVariantStyle},
    system::{add_interval, add_timer, send_window_request, with_system},
    text_editor::TextEditor,
    timer::TimerId,
    types::{Point, Rect},
    window::SetFocusRequest,
};

use super::{Widget, WidgetCommon, WidgetExt};

// TODO: pub(crate)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role1 {
    Default,
    ScrollLeft,
    ScrollRight,
    ScrollUp,
    ScrollDown,
    ScrollGripX,
    ScrollGripY,
    ScrollPager,
}

pub struct Button {
    editor: TextEditor,
    icon: Option<Rc<Pixmap>>,
    text_visible: bool,
    auto_repeat: bool,
    is_mouse_leave_sensitive: bool,
    trigger_on_press: bool,
    on_triggered: CallbackVec<String>,
    is_pressed: bool,
    was_pressed_but_moved_out: bool,
    auto_repeat_delay_timer: Option<TimerId>,
    auto_repeat_interval: Option<TimerId>,
    common: WidgetCommon,
    role: Role1,
}

#[impl_with]
impl Button {
    pub fn new(text: impl Display) -> Self {
        let mut common = WidgetCommon::new();
        common.set_focusable(true);
        let mut editor = TextEditor::new(&text.to_string());
        editor.set_cursor_hidden(true);
        Self {
            editor,
            icon: None,
            text_visible: true,
            auto_repeat: false,
            is_mouse_leave_sensitive: true,
            trigger_on_press: false,
            on_triggered: CallbackVec::new(),
            is_pressed: false,
            was_pressed_but_moved_out: false,
            common,
            role: Role1::Default,
            auto_repeat_delay_timer: None,
            auto_repeat_interval: None,
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.editor.set_text(&text.to_string(), Attrs::new());
        self.common.size_hint_changed();
        self.common.update();
    }

    pub fn set_text_visible(&mut self, value: bool) {
        self.text_visible = value;
        self.common.size_hint_changed();
        self.common.update();
    }

    pub fn set_auto_repeat(&mut self, value: bool) {
        self.auto_repeat = value;
    }

    pub fn set_mouse_leave_sensitive(&mut self, value: bool) {
        self.is_mouse_leave_sensitive = value;
    }

    pub fn set_trigger_on_press(&mut self, value: bool) {
        self.trigger_on_press = value;
    }

    // TODO: set_icon should preferably work with SVG icons
    // pub fn set_icon(&mut self, icon: Option<Rc<Pixmap>>) {
    //     self.icon = icon;
    //     self.common.size_hint_changed();
    //     self.common.update();
    // }

    pub fn on_triggered(&mut self, callback: Callback<String>) {
        self.on_triggered.push(callback);
    }

    pub fn trigger(&mut self) {
        self.on_triggered.invoke(self.editor.text());
    }

    fn current_style(&self) -> &ComputedStyle {
        match self.role {
            Role1::Default => &self.common.style().0.button,
            Role1::ScrollLeft => &self.common.style().0.scroll_bar.scroll_left,
            Role1::ScrollRight => &self.common.style().0.scroll_bar.scroll_right,
            Role1::ScrollUp => &self.common.style().0.scroll_bar.scroll_up,
            Role1::ScrollDown => &self.common.style().0.scroll_bar.scroll_down,
            Role1::ScrollGripX => &self.common.style().0.scroll_bar.scroll_grip_x,
            Role1::ScrollGripY => &self.common.style().0.scroll_bar.scroll_grip_y,
            Role1::ScrollPager => &self.common.style().0.scroll_bar.scroll_pager,
        }
    }

    fn current_variant_style(&self) -> &ComputedVariantStyle {
        let state = if self.common.is_enabled() {
            ButtonState::Enabled {
                focused: self.common.is_focused(),
                mouse_over: self.common.is_mouse_over,
                pressed: self.is_pressed,
            }
        } else {
            ButtonState::Disabled
        };
        self.current_style().variants.get(&state).unwrap()
    }

    // TODO: pub(crate)
    pub fn set_role(&mut self, role: Role1) {
        self.role = role;
        self.icon = self.current_variant_style().icon.clone();
        self.common.set_focusable(role == Role1::Default);
        self.common.size_hint_changed();
        self.common.update();
    }

    fn set_pressed(&mut self, value: bool, suppress_trigger: bool) {
        if self.is_pressed == value {
            return;
        }
        self.is_pressed = value;
        self.common.update();
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
}

impl Widget for Button {
    impl_widget_common!();

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        let size = self.common.size_or_err()?;
        let style = self.current_variant_style().clone();

        event.stroke_and_fill_rounded_rect(
            Rect {
                top_left: Point::default(),
                size,
            },
            &style.border,
            style.background.as_ref(),
        );

        if self.text_visible {
            self.editor.set_text_color(style.text_color);
            let editor_pixmap = self.editor.pixmap();
            let padding = Point {
                x: max(0, size.x - editor_pixmap.width() as i32) / 2,
                y: max(0, size.y - editor_pixmap.height() as i32) / 2,
            };
            event.draw_pixmap(padding, editor_pixmap.as_ref(), Default::default());
        }

        // TODO: display icon and text side by side if both are present
        if let Some(icon) = &self.icon {
            let pos = Point {
                x: max(0, size.x - icon.width() as i32) / 2,
                y: max(0, size.y - icon.height() as i32) / 2,
            };
            event.draw_pixmap(pos, (**icon).as_ref(), Default::default());
        }
        Ok(())
    }

    fn handle_mouse_move(&mut self, event: MouseMoveEvent) -> Result<bool> {
        let rect = self.common.rect_or_err()?;
        if rect.contains(event.pos()) {
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
        if event.button() == MouseButton::Left {
            if event.state().is_pressed() {
                self.set_pressed(true, false);
                if !self.common.is_focused() {
                    let window = self.common.window_or_err()?;
                    if self.role == Role1::Default {
                        send_window_request(
                            window.id(),
                            SetFocusRequest {
                                widget_id: self.common.id,
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
        if event.info().physical_key == PhysicalKey::Code(KeyCode::Space)
            || event.info().logical_key == Key::Named(NamedKey::Space)
        {
            self.set_pressed(event.info().state.is_pressed(), false);
            return Ok(true);
        }
        if event.info().physical_key == PhysicalKey::Code(KeyCode::Enter)
            || event.info().physical_key == PhysicalKey::Code(KeyCode::NumpadEnter)
            || event.info().logical_key == Key::Named(NamedKey::Enter)
        {
            self.trigger();
            return Ok(true);
        }
        Ok(false)
    }

    fn handle_accessible_action(&mut self, event: AccessibleActionEvent) -> Result<()> {
        if self.role != Role1::Default {
            warn!("unexpected accessible action for role: {:?}", self.role);
            return Ok(());
        }

        match event.action() {
            Action::Default => self.trigger(),
            Action::Focus => {
                send_window_request(
                    self.common.window_or_err()?.id(),
                    SetFocusRequest {
                        widget_id: self.common.id,
                        // TODO: separate reason?
                        reason: FocusReason::Mouse,
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }

    fn accessible_node(&mut self) -> Option<accesskit::NodeBuilder> {
        if self.role != Role1::Default {
            return None;
        }

        let mut node = NodeBuilder::new(Role::Button);
        node.set_name(self.editor.text().as_str());
        node.add_action(Action::Focus);
        //node.add_action(Action::Default);
        node.set_default_action_verb(DefaultActionVerb::Click);
        Some(node)
    }

    fn recalculate_size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let style = &self.common.style().0.button;
        let padding = match mode {
            SizeHintMode::Min => style.min_padding_with_border,
            SizeHintMode::Preferred => style.preferred_padding_with_border,
        };

        // TODO: support text with icon
        let content_size = if self.text_visible {
            self.editor.size().x
        } else if let Some(icon) = &self.icon {
            icon.width() as i32
        } else {
            0
        };

        Ok(content_size + 2 * padding.x)
    }

    fn recalculate_size_hint_y(&mut self, _size_x: i32, mode: SizeHintMode) -> Result<i32> {
        // TODO: use size_x, handle multiple lines
        let style = &self.common.style().0.button;
        let padding = match mode {
            SizeHintMode::Min => style.min_padding_with_border,
            SizeHintMode::Preferred => style.preferred_padding_with_border,
        };

        // TODO: support text with icon
        let content_size = if self.text_visible {
            self.editor.size().y
        } else if let Some(icon) = &self.icon {
            icon.height() as i32
        } else {
            0
        };

        Ok(content_size + 2 * padding.x)
    }

    fn handle_widget_scope_change(&mut self, _event: WidgetScopeChangeEvent) -> Result<()> {
        self.editor
            .set_font_metrics(self.current_style().font_metrics);
        self.icon = self.current_variant_style().icon.clone();
        if !self.common.is_enabled() {
            if let Some(id) = self.auto_repeat_delay_timer.take() {
                id.cancel();
            }
            if let Some(id) = self.auto_repeat_interval.take() {
                id.cancel();
            }
            self.is_pressed = false;
            self.was_pressed_but_moved_out = false;
        }
        self.common.size_hint_changed();
        self.common.update();
        Ok(())
    }
}
