use {
    crate::clean::widgets::{base::WidgetBase, focusable::FocusableAttrs},
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Button {
    base: WidgetBase,

    focusable: FocusableAttrs,
    // TODO: focusable: FocusableAttributes,
    #[widgem_attr(constructor)]
    text: String,
    #[widgem_attr(default = true)]
    auto_repeat: bool,
    #[widgem_attr(default = true)]
    is_mouse_leave_sensitive: bool,
    #[widgem_attr(default = true)]
    trigger_on_press: bool,

    #[widgem_attr(private)]
    is_pressed: bool,
    #[widgem_attr(private)]
    was_pressed_but_moved_out: bool,
}

#[test]
fn t1() {
    Button::new("abc".into()).auto_repeat(false);
}
