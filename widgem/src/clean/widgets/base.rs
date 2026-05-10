use {strict_num::FiniteF32, widgem_macros::AttributeSetters, winit::window::CursorIcon};

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
#[widgem_attr(extension_trait = WidgetBaseExt)]
pub struct WidgetBase {
    /// CSS properties.
    style: Option<String>,
    #[widgem_attr(default = true)]
    enabled: bool,
    #[widgem_attr(default = true)]
    visible: bool,
    /// This attribute is ignored for widgets that do not expose an accessibility node.
    #[widgem_attr(default = true)]
    accessibility_node_enabled: bool,
    // effective icon is attrs.cursor_icon OR default widget icon OR parent icon
    cursor_icon: Option<CursorIcon>,
    scale: Option<FiniteF32>,
}
