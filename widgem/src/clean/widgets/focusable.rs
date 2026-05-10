use widgem_macros::AttributeSetters;

/// Common attributes for all focusable widgets.
///
/// Note that `is_focused` is not a widget attribute because it cannot be
/// independently set for each widget.
#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
#[widgem_attr(extension_trait = FocusableExt)]
pub struct FocusableAttrs {
    /// By default, widgets that support focus are focusable.
    /// This attribute is ignored for widgets that do not support focus.
    #[widgem_attr(default = true)]
    focusable: bool,
    // TODO: tab_order
}
