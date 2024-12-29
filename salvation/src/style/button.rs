use {
    super::{
        computed::{ComputedElementStyle, ComputedStyle},
        css::{Element, MyPseudoClass},
        ElementState,
    },
    crate::style::css::{convert_content_url, convert_zoom},
    itertools::Itertools,
    log::warn,
    std::rc::Rc,
    tiny_skia::Pixmap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    Enabled {
        focused: bool,
        mouse_over: bool,
        pressed: bool,
    },
    Disabled,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::Enabled {
            focused: false,
            mouse_over: false,
            pressed: false,
        }
    }
}

impl ButtonState {
    pub fn element(&self) -> Element {
        let mut element = Element::new("button");
        match self {
            Self::Enabled {
                focused,
                mouse_over,
                pressed,
            } => {
                element.add_pseudo_class(MyPseudoClass::Enabled);
                if *focused {
                    element.add_pseudo_class(MyPseudoClass::Focus);
                }
                if *mouse_over {
                    element.add_pseudo_class(MyPseudoClass::Hover);
                }
                if *pressed {
                    element.add_pseudo_class(MyPseudoClass::Active);
                }
            }
            Self::Disabled => {
                element.add_pseudo_class(MyPseudoClass::Disabled);
            }
        }
        element
    }
}

impl ElementState for ButtonState {
    fn all() -> Vec<Self> {
        let all_bools = [false, true];
        let mut r = all_bools
            .into_iter()
            .cartesian_product(all_bools)
            .cartesian_product(all_bools)
            .map(|((focused, mouse_over), pressed)| Self::Enabled {
                focused,
                mouse_over,
                pressed,
            })
            .collect_vec();
        r.push(Self::Disabled);
        r
    }
}

#[derive(Debug, Clone, Default)]
pub struct ComputedButtonStyle {
    pub icon: Option<Rc<Pixmap>>,
}

impl ComputedElementStyle for ComputedButtonStyle {
    fn new(style: &ComputedStyle, element: &Element) -> ComputedButtonStyle {
        let properties = style.0.style.find_rules(|s| element.matches(s));

        let scale = style.0.scale * convert_zoom(&properties);
        let mut icon = None;
        if let Some(url) = convert_content_url(&properties) {
            //println!("icon url: {url:?}");
            match style.0.style.load_pixmap(&url, scale) {
                Ok(pixmap) => icon = Some(pixmap),
                Err(err) => warn!("failed to load icon: {err:?}"),
            }
        }
        Self { icon }
    }
}
