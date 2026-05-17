use {
    crate::clean::{
        widgets::{Empty, WidgetBase, WidgetBaseExt},
        BoxWidget, Widget, WidgetExt,
    },
    widgem_macros::AttributeSetters,
};

#[derive(Debug)]
struct NativeWindow {
    window: winit::window::Window,
}

#[derive(Debug, AttributeSetters)]
pub struct Window {
    #[widgem_attr(extend = WidgetBaseExt)]
    base: WidgetBase,
    #[widgem_attr(constructor)]
    title: String,
    content: Option<BoxWidget>,

    #[widgem_attr(private)]
    pressed: bool,
    #[widgem_attr(private)]
    native_window: Option<NativeWindow>,
}

// TODO: derive Clone, PartialEq, Eq, Hash in AttributeSetters macro?
impl Clone for Window {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            title: self.title.clone(),
            content: self.content.clone(),
            pressed: self.pressed,
            native_window: None,
        }
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
            && self.title == other.title
            && self.content == other.content
            && self.pressed == other.pressed
            && self.native_window.is_none()
            && other.native_window.is_none()
    }
}

impl Eq for Window {}

impl std::hash::Hash for Window {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.hash(state);
        self.title.hash(state);
        self.content.hash(state);
        self.pressed.hash(state);
    }
}

impl Widget for Window {
    fn render(&self, _ctx: crate::clean::WidgetRenderContext) -> BoxWidget {
        if let Some(content) = &self.content {
            content.clone()
        } else {
            Empty::new().boxed()
        }
    }
}
