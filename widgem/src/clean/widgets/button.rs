use {
    crate::clean::widgets::{
        base::{WidgetBase, WidgetBaseExt},
        focusable::{FocusableAttrs, FocusableExt},
    },
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Button {
    #[widgem_attr(extend = WidgetBaseExt)]
    base: WidgetBase,
    #[widgem_attr(extend = FocusableExt)]
    focusable: FocusableAttrs,
    // TODO: focusable: FocusableAttributes,
    #[widgem_attr(constructor)]
    text: String,
    #[widgem_attr(default = true)]
    auto_repeat: bool,
    #[widgem_attr(default = true)]
    mouse_leave_sensitive: bool,
    #[widgem_attr(default = true)]
    trigger_on_press: bool,

    #[widgem_attr(private)]
    pressed: bool,
    #[widgem_attr(private)]
    was_pressed_but_moved_out: bool,
}

#[test]
fn t1() {
    Button::new("abc".into())
        .auto_repeat(false)
        .enabled(false)
        .focusable(true);
}
