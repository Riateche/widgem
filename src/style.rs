use tiny_skia::Color;

#[derive(Debug)]
pub struct Palette {
    pub foreground: Color,
    pub background: Color,
    pub unfocused_input_border: Color,
    pub focused_input_border: Color,
}

#[derive(Debug)]
pub struct Style {
    pub palette: Palette,
}
