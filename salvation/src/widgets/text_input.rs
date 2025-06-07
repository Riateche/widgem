use {
    super::{Widget, WidgetAddress, WidgetCommon, WidgetCommonTyped, WidgetExt, WidgetGeometry},
    crate::{
        draw::DrawEvent,
        event::{
            FocusInEvent, FocusOutEvent, InputMethodEvent, KeyboardInputEvent, LayoutEvent,
            ScrollToRectRequest, StyleChangeEvent,
        },
        impl_widget_common,
        layout::{
            grid::{grid_layout, GridAxisOptions, GridOptions},
            Alignment, SizeHints,
        },
        style::text_input::{ComputedVariantStyle, TextInputState},
        text_editor::Text,
        types::{PhysicalPixels, Point, PpxSuffix, Rect},
    },
    anyhow::Result,
    cosmic_text::Attrs,
    log::warn,
    std::{cmp::max, fmt::Display},
    winit::window::CursorIcon,
};

struct Viewport {
    common: WidgetCommonTyped<Self>,
}

impl Widget for Viewport {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self { common }
    }

    fn handle_size_hint_x_request(&mut self) -> Result<crate::layout::SizeHints> {
        Ok(SizeHints {
            min: 0.ppx(),
            preferred: 0.ppx(),
            is_fixed: false,
        })
    }

    fn handle_size_hint_y_request(&mut self, _size_x: PhysicalPixels) -> Result<SizeHints> {
        let size = PhysicalPixels::from_i32(
            self.common.get_child::<Text>(0).unwrap().line_height() as i32,
        );
        Ok(SizeHints {
            min: size,
            preferred: size,
            is_fixed: true,
        })
    }
}

pub struct TextInput {
    common: WidgetCommonTyped<Self>,
}

impl TextInput {
    fn text_widget(&self) -> &Text {
        self.common
            .get_dyn_child(0)
            .unwrap()
            .common()
            .get_child::<Text>(0)
            .unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.common
            .get_dyn_child_mut(0)
            .unwrap()
            .common_mut()
            .get_child_mut::<Text>(0)
            .unwrap()
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text_widget_mut().set_text(text, Attrs::new());
    }

    fn adjust_scroll(&mut self, changed_size_hints: &[WidgetAddress]) {
        let Some(editor_viewport_rect) = self
            .common
            .children
            .get(&0.into())
            .unwrap()
            .common()
            .rect_in_parent()
        else {
            return;
        };
        let text_size = self.text_widget().size();
        let cursor_position = self.text_widget().cursor_position();
        let mut scroll_x = self
            .common
            .get_dyn_child(0)
            .unwrap()
            .common()
            .children
            .get(&0.into())
            .unwrap()
            .common()
            .rect_in_parent()
            .map_or(0.ppx(), |rect| -rect.left());
        let max_scroll = max(0.ppx(), text_size.x() - editor_viewport_rect.size_x());
        if let Some(cursor_position) = cursor_position {
            let cursor_x_in_viewport = cursor_position.x() - scroll_x;
            if cursor_x_in_viewport < 0.ppx() {
                scroll_x -= -cursor_x_in_viewport;
            } else if cursor_x_in_viewport > editor_viewport_rect.size_x() - 1.ppx() {
                scroll_x += cursor_x_in_viewport - (editor_viewport_rect.size_x() - 1.ppx());
            }
        }
        scroll_x = scroll_x.clamp(0.ppx(), max_scroll);
        let new_rect = Rect::from_pos_size(Point::new(-scroll_x, 0.ppx()), text_size);
        if self
            .common
            .get_dyn_child(0)
            .unwrap()
            .common()
            .children
            .get(&0.into())
            .unwrap()
            .common()
            .rect_in_parent()
            != Some(new_rect)
        {
            let Some(geometry) = self
                .common
                .get_dyn_child(0)
                .unwrap()
                .common()
                .geometry
                .clone()
            else {
                return;
            };

            self.common
                .get_dyn_child_mut(0)
                .unwrap()
                .common_mut()
                .children
                .get_mut(&0.into())
                .unwrap()
                .set_geometry(
                    Some(WidgetGeometry::new(&geometry, new_rect)),
                    changed_size_hints,
                );
        }
    }

    fn current_variant_style(&self) -> &ComputedVariantStyle {
        let state = if self.common.is_enabled() {
            TextInputState::Enabled {
                focused: self.common.is_focused(),
                mouse_over: self.common.is_mouse_over,
            }
        } else {
            TextInputState::Disabled
        };
        self.common
            .style()
            .0
            .text_input
            .variants
            .get(&state)
            .unwrap()
    }

    fn refresh_style(&mut self) {
        let style = self.common.style().0.text_input.clone();
        let variant_style = self.current_variant_style().clone();
        self.common.set_grid_options(Some(GridOptions {
            x: GridAxisOptions {
                min_padding: style.min_padding_with_border.x(),
                min_spacing: 0.ppx(),
                preferred_padding: style.preferred_padding_with_border.x(),
                preferred_spacing: 0.ppx(),
                border_collapse: 0.ppx(),
                alignment: Alignment::Start,
            },
            y: GridAxisOptions {
                min_padding: style.min_padding_with_border.y(),
                min_spacing: 0.ppx(),
                preferred_padding: style.preferred_padding_with_border.y(),
                preferred_spacing: 0.ppx(),
                border_collapse: 0.ppx(),
                alignment: Alignment::Start,
            },
        }));
        let text_widget = self.text_widget_mut();
        text_widget.set_font_metrics(style.font_metrics);
        // TODO: support color changes based on state
        text_widget.set_text_color(variant_style.text_color);
        text_widget.set_selected_text_color(variant_style.selected_text_color);
        text_widget.set_selected_text_background(variant_style.selected_text_background);
    }
}

impl Widget for TextInput {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        common.set_supports_focus(true);
        common.cursor_icon = CursorIcon::Text;
        let host_id = common.id();
        let viewport = common
            .add_child_with_key::<Viewport>(0)
            .set_column(0)
            .set_row(0);
        viewport.common_mut().set_receives_all_mouse_events(true);
        viewport.common_mut().cursor_icon = CursorIcon::Text;
        let editor = viewport
            .common_mut()
            .add_child_with_key::<Text>(0)
            .set_multiline(false)
            .set_editable(true)
            .set_host_id(host_id.into());
        editor.common_mut().set_receives_all_mouse_events(true);
        let mut t = Self { common };
        t.refresh_style();
        t
    }

    fn handle_focus_in(&mut self, event: FocusInEvent) -> Result<()> {
        self.text_widget_mut().handle_host_focus_in(event.reason)
    }

    fn handle_focus_out(&mut self, _event: FocusOutEvent) -> Result<()> {
        self.text_widget_mut().handle_host_focus_out()
    }

    fn handle_layout(&mut self, event: LayoutEvent) -> Result<()> {
        grid_layout(self, &event.changed_size_hints);
        self.adjust_scroll(&event.changed_size_hints);
        Ok(())
    }

    fn handle_style_change(&mut self, _event: StyleChangeEvent) -> Result<()> {
        self.refresh_style();

        Ok(())
    }

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        let rect_in_window = self.common.rect_in_window_or_err()?;
        let style = self.current_variant_style();

        // TODO: stroke and fill instead
        event.stroke_rounded_rect(
            Rect::from_pos_size(Point::default(), rect_in_window.size()),
            style.border.radius.to_i32() as f32,
            style.border.color,
            style.border.width.to_i32() as f32,
        );

        Ok(())
    }

    fn handle_size_hint_x_request(&mut self) -> Result<SizeHints> {
        let style = &self.common.style().0.text_input;
        Ok(SizeHints {
            min: style.min_width,
            preferred: style.preferred_width,
            is_fixed: false,
        })
    }

    fn handle_size_hint_y_request(&mut self, _size_x: PhysicalPixels) -> Result<SizeHints> {
        let text_size = self.text_widget().size();
        let style = &self.common.style().0.text_input;
        Ok(SizeHints {
            min: text_size.y() + 2 * style.min_padding_with_border.y(),
            preferred: text_size.y() + 2 * style.preferred_padding_with_border.y(),
            is_fixed: true,
        })
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        self.text_widget_mut().handle_host_keyboard_input(event)
    }

    fn handle_input_method(&mut self, event: InputMethodEvent) -> Result<bool> {
        self.text_widget_mut().handle_host_ime(event)
    }

    fn handle_scroll_to_rect_request(&mut self, event: ScrollToRectRequest) -> Result<bool> {
        if self.text_widget().common().id() != event.address.widget_id() {
            warn!("TextInput received unexpected ScrollToRectEvent");
            return Ok(false);
        }

        self.adjust_scroll(&[]);

        Ok(true)
    }
}
