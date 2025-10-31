use {
    crate::{
        event::{
            AccessibilityActionEvent, FocusInEvent, FocusOutEvent, FocusReason, InputMethodEvent,
            KeyboardInputEvent, StyleChangeEvent,
        },
        impl_widget_base,
        layout::{default_size_hint_x, default_size_hint_y, SizeHint},
        style::{
            common::ComputedElementStyle,
            css::{convert_font, convert_height, convert_width, PseudoClass, StyleSelector},
            defaults::{DEFAULT_MIN_WIDTH_EM, DEFAULT_PREFERRED_WIDTH_EM},
            Styles,
        },
        system::OrWarn,
        text::TextHandler,
        types::PhysicalPixels,
        widget_initializer::{self, WidgetInitializer},
        widgets::{Row, ScrollArea},
        ChildKey, Widget, WidgetBaseOf, WidgetExt,
    },
    accesskit::ActionData,
    anyhow::{bail, Result},
    std::{fmt::Display, rc::Rc},
    tracing::warn,
    winit::window::CursorIcon,
};

pub struct TextArea {
    base: WidgetBaseOf<Self>,
    style: Rc<TextAreaStyle>,
    expand_to_fit_content_x: bool,
    expand_to_fit_content_y: bool,
}

impl TextArea {
    fn new(mut base: WidgetBaseOf<Self>) -> anyhow::Result<Self> {
        // TODO: set ime enabled
        base.set_supports_focus(true);
        base.set_cursor_icon(CursorIcon::Text);
        let host_id = base.id();
        let text_style = base.compute_style();
        base.set_main_child(ScrollArea::init())?
            .set_size_x_fixed(Some(false))
            .set_size_y_fixed(Some(false))
            .set_content(Row::init())?
            .add_class("text_area_text_wrapper".into())
            .contents_mut()
            .set_next_item(TextHandler::init(String::new(), text_style))?
            .set_multiline(true)
            .set_editable(true)
            .set_host_id(host_id.into())
            .set_size_x_fixed(Some(false))
            .set_size_y_fixed(Some(false));
        Ok(TextArea {
            style: base.compute_style(),
            expand_to_fit_content_x: false,
            expand_to_fit_content_y: false,
            base,
        })
    }

    // TODO: name or label ref?
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_fallible_new(Self::new)
    }

    // fn text_handler(&self) -> anyhow::Result<&TextHandler> {
    //     self.base
    //         .get_child::<ScrollArea>(ChildKey::main())?
    //         .content::<Row>()?
    //         .base()
    //         .get_child(0u32)
    // }

    fn text_handler_mut(&mut self) -> anyhow::Result<&mut TextHandler> {
        self.base
            .get_child_mut::<ScrollArea>(ChildKey::main())?
            .content_mut::<Row>()?
            .base_mut()
            .get_child_mut(0u32)
    }

    pub fn set_text(&mut self, text: impl Display) {
        let Some(handler) = self.text_handler_mut().or_warn() else {
            return;
        };
        handler.set_text(text);
    }

    pub fn set_expand_to_fit_content_x(&mut self, value: bool) -> &mut Self {
        if self.expand_to_fit_content_x != value {
            self.expand_to_fit_content_x = value;
            self.base.size_hint_changed();
        }
        self
    }

    pub fn set_expand_to_fit_content_y(&mut self, value: bool) -> &mut Self {
        if self.expand_to_fit_content_y != value {
            self.expand_to_fit_content_y = value;
            self.base.size_hint_changed();
        }
        self
    }

    pub fn set_expand_to_fit_content(&mut self, value: bool) -> &mut Self {
        self.set_expand_to_fit_content_x(value)
            .set_expand_to_fit_content_y(value)
    }
}

impl Widget for TextArea {
    impl_widget_base!();

    fn handle_focus_in(&mut self, event: FocusInEvent) -> Result<()> {
        self.text_handler_mut()?.handle_host_focus_in(event.reason)
    }

    fn handle_focus_out(&mut self, _event: FocusOutEvent) -> Result<()> {
        self.text_handler_mut()?.handle_host_focus_out()
    }

    fn handle_style_change(&mut self, _event: StyleChangeEvent) -> Result<()> {
        self.style = self.base.compute_style();
        Ok(())
    }

    fn handle_size_hint_x_request(&self, size_y: Option<PhysicalPixels>) -> Result<SizeHint> {
        if self.expand_to_fit_content_x {
            Ok(default_size_hint_x(self, size_y))
        } else {
            Ok(SizeHint::new_expanding(
                self.style.min_width,
                self.style.preferred_width,
            ))
        }
    }

    fn handle_size_hint_y_request(&self, size_x: PhysicalPixels) -> Result<SizeHint> {
        if self.expand_to_fit_content_y {
            Ok(default_size_hint_y(self, size_x))
        } else {
            Ok(SizeHint::new_expanding(
                self.style.min_height,
                self.style.preferred_height,
            ))
        }
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        self.text_handler_mut()?.handle_host_keyboard_input(event)
    }

    fn handle_input_method(&mut self, event: InputMethodEvent) -> Result<bool> {
        self.text_handler_mut()?.handle_host_ime(event)
    }

    fn handle_accessibility_node_request(&mut self) -> Result<Option<accesskit::Node>> {
        self.text_handler_mut()?
            .handle_host_accessibility_node_request()
    }

    fn handle_accessibility_action(&mut self, event: AccessibilityActionEvent) -> Result<bool> {
        match event.action {
            accesskit::Action::Click => {
                self.base.set_focus(FocusReason::Mouse);
                Ok(true)
            }
            accesskit::Action::SetValue => {
                let value: String = match event.data {
                    Some(ActionData::Value(value)) => value.into(),
                    Some(ActionData::NumericValue(value)) => value.to_string(),
                    _ => bail!(
                        "expected Value or NumericValue in data, got {:?}",
                        event.data
                    ),
                };
                self.set_text(value);
                Ok(true)
            }
            accesskit::Action::SetTextSelection => {
                let Some(ActionData::SetTextSelection(data)) = event.data else {
                    bail!("expected SetTextSelection in data, got {:?}", event.data);
                };
                self.text_handler_mut()?
                    .handle_accessibility_set_selection_action(data);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextAreaStyle {
    pub min_width: PhysicalPixels,
    pub preferred_width: PhysicalPixels,
    pub min_height: PhysicalPixels,
    pub preferred_height: PhysicalPixels,
}

impl ComputedElementStyle for TextAreaStyle {
    fn new(style: &Styles, element: &StyleSelector, scale: f32) -> TextAreaStyle {
        let element_min = element
            .clone()
            .with_pseudo_class(PseudoClass::Custom("min".into()));

        let properties = style.find_rules(|s| element.matches(s));
        let font = convert_font(&properties, Some(&style.root_font_style()));
        let preferred_width = convert_width(&properties, scale, font.font_size)
            .or_warn()
            .flatten()
            .unwrap_or_else(|| {
                warn!("missing width in text area css");
                (font.font_size * DEFAULT_PREFERRED_WIDTH_EM).to_physical(scale)
            });
        let preferred_height = convert_height(&properties, scale, font.font_size)
            .or_warn()
            .flatten()
            .unwrap_or_else(|| {
                warn!("missing height in text area css");
                (font.font_size * DEFAULT_PREFERRED_WIDTH_EM).to_physical(scale)
            });

        let min_properties = style.find_rules(|s| element_min.matches(s));
        let min_width = convert_width(&min_properties, scale, font.font_size)
            .or_warn()
            .flatten()
            .unwrap_or_else(|| {
                warn!("missing width in text area min css");
                (font.font_size * DEFAULT_MIN_WIDTH_EM).to_physical(scale)
            });
        let min_height = convert_height(&min_properties, scale, font.font_size)
            .or_warn()
            .flatten()
            .unwrap_or_else(|| {
                warn!("missing height in text area min css");
                (font.font_size * DEFAULT_MIN_WIDTH_EM).to_physical(scale)
            });

        Self {
            min_width,
            preferred_width,
            min_height,
            preferred_height,
        }
    }
}
