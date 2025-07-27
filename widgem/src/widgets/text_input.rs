use {
    super::{Widget, WidgetAddress, WidgetBaseOf, WidgetExt, WidgetGeometry},
    crate::{
        event::{
            FocusInEvent, FocusOutEvent, InputMethodEvent, KeyboardInputEvent, LayoutEvent,
            StyleChangeEvent,
        },
        impl_widget_base,
        layout::{grid::grid_layout, SizeHints},
        style::{
            common::ComputedElementStyle,
            css::{convert_font, convert_width, PseudoClass, StyleSelector},
            defaults::{DEFAULT_MIN_WIDTH_EM, DEFAULT_PREFERRED_WIDTH_EM},
            get_style, Style,
        },
        system::ReportError,
        text_editor::Text,
        types::{PhysicalPixels, Point, PpxSuffix, Rect},
        widgets::NewWidget,
        ScrollToRectRequest,
    },
    anyhow::Result,
    cosmic_text::Attrs,
    log::warn,
    std::{cmp::max, fmt::Display, rc::Rc},
    winit::window::CursorIcon,
};

struct Viewport {
    base: WidgetBaseOf<Self>,
}

impl NewWidget for Viewport {
    type Arg = ();

    fn new(base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for Viewport {
    impl_widget_base!();

    fn handle_size_hint_x_request(&self) -> Result<crate::layout::SizeHints> {
        Ok(SizeHints {
            min: 0.ppx(),
            preferred: 0.ppx(),
            is_fixed: false,
        })
    }

    fn handle_size_hint_y_request(&self, _size_x: PhysicalPixels) -> Result<SizeHints> {
        let size =
            PhysicalPixels::from_i32(self.base.get_child::<Text>(0).unwrap().line_height() as i32);
        Ok(SizeHints {
            min: size,
            preferred: size,
            is_fixed: true,
        })
    }
}

pub struct TextInput {
    base: WidgetBaseOf<Self>,
    style: Rc<TextInputStyle>,
}

impl TextInput {
    fn text_widget(&self) -> &Text {
        self.base
            .get_dyn_child(0)
            .unwrap()
            .base()
            .get_child::<Text>(0)
            .unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.base
            .get_dyn_child_mut(0)
            .unwrap()
            .base_mut()
            .get_child_mut::<Text>(0)
            .unwrap()
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text_widget_mut().set_text(text, Attrs::new());
    }

    fn adjust_scroll(&mut self, changed_size_hints: &[WidgetAddress]) {
        let Some(editor_viewport_rect) =
            self.base.get_dyn_child(0).unwrap().base().rect_in_parent()
        else {
            return;
        };
        let text_size = self.text_widget().size();
        let cursor_position = self.text_widget().cursor_position();
        let mut scroll_x = self
            .base
            .get_dyn_child(0)
            .unwrap()
            .base()
            .get_dyn_child(0)
            .unwrap()
            .base()
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
            .base
            .get_dyn_child(0)
            .unwrap()
            .base()
            .get_dyn_child(0)
            .unwrap()
            .base()
            .rect_in_parent()
            != Some(new_rect)
        {
            let Some(geometry) = self
                .base
                .get_dyn_child(0)
                .unwrap()
                .base()
                .geometry()
                .cloned()
            else {
                return;
            };

            self.base
                .get_dyn_child_mut(0)
                .unwrap()
                .base_mut()
                .get_dyn_child_mut(0)
                .unwrap()
                .set_geometry(
                    Some(WidgetGeometry::new(&geometry, new_rect)),
                    changed_size_hints,
                );
        }
    }
}

impl NewWidget for TextInput {
    // TODO: name or label ref?
    type Arg = ();

    fn new(mut base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        base.set_supports_focus(true);
        base.set_cursor_icon(CursorIcon::Text);
        let host_id = base.id();
        let element = base.style_selector().clone();
        let viewport = base.add_child_with_key::<Viewport>(0, ());
        viewport.base_mut().set_receives_all_mouse_events(true);
        viewport.base_mut().set_cursor_icon(CursorIcon::Text);
        let editor = viewport
            .base_mut()
            .add_child_with_key::<Text>(0, String::new())
            .set_multiline(false)
            .set_editable(true)
            .set_host_id(host_id.into())
            .set_host_style_selector(element);
        editor.base_mut().set_receives_all_mouse_events(true);
        Self {
            style: get_style(base.style_selector(), base.scale()),
            base,
        }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for TextInput {
    impl_widget_base!();

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
        self.style = get_style(self.base().style_selector(), self.base().scale());
        Ok(())
    }

    fn handle_size_hint_x_request(&self) -> Result<SizeHints> {
        Ok(SizeHints {
            min: self.style.min_width,
            preferred: self.style.preferred_width,
            is_fixed: false,
        })
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        self.text_widget_mut().handle_host_keyboard_input(event)
    }

    fn handle_input_method(&mut self, event: InputMethodEvent) -> Result<bool> {
        self.text_widget_mut().handle_host_ime(event)
    }

    fn handle_scroll_to_rect_request(&mut self, event: ScrollToRectRequest) -> Result<bool> {
        if self.text_widget().base().id() != event.address.widget_id() {
            warn!("TextInput received unexpected ScrollToRectEvent");
            return Ok(false);
        }

        self.adjust_scroll(&[]);

        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct TextInputStyle {
    pub min_width: PhysicalPixels,
    pub preferred_width: PhysicalPixels,
}

impl ComputedElementStyle for TextInputStyle {
    fn new(style: &Style, element: &StyleSelector, scale: f32) -> TextInputStyle {
        let element_min = element
            .clone()
            .with_pseudo_class(PseudoClass::Custom("min".into()));

        let properties = style.find_rules(|s| element.matches(s));
        let font = convert_font(&properties, Some(&style.root_font_style()));
        let preferred_width = convert_width(&properties, scale, font.font_size)
            .or_report_err()
            .flatten()
            .unwrap_or_else(|| {
                warn!("missing width in text input css");
                (font.font_size * DEFAULT_PREFERRED_WIDTH_EM).to_physical(scale)
            });

        let min_properties = style.find_rules(|s| element_min.matches(s));
        let min_width = convert_width(&min_properties, scale, font.font_size)
            .or_report_err()
            .flatten()
            .unwrap_or_else(|| {
                warn!("missing width in text input min css");
                (font.font_size * DEFAULT_MIN_WIDTH_EM).to_physical(scale)
            });

        Self {
            min_width,
            preferred_width,
        }
    }
}
