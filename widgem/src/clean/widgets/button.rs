use widgem_macros::AttributeSetters;

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Button {
    // TODO: widget: CommonWidgetAttributes,
    // TODO: focusable: FocusableAttributes,
    #[widgem_attr(constructor)]
    text: String,
    #[widgem_attr(default = true)]
    auto_repeat: bool,
    #[widgem_attr(default = true)]
    is_mouse_leave_sensitive: bool,
    #[widgem_attr(default = true)]
    trigger_on_press: bool,
}

#[test]
fn t1() {
    Button::new("abc".into()).auto_repeat(false);
}
