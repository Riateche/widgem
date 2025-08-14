use {
    crate::{
        style::{
            common::{ComputedBackground, ComputedBorderStyle},
            RelativeOffset,
        },
        types::{Point, PpxSuffix, Rect},
    },
    std::{cell::RefCell, rc::Rc},
    tiny_skia::{
        BlendMode, Color, FillRule, FilterQuality, LinearGradient, Mask, Paint, Path, PathBuilder,
        Pattern, Pixmap, PixmapPaint, PixmapRef, Shader, SpreadMode, Stroke, Transform,
    },
    tracing::warn,
};

fn rounded_line_in_square_corner(
    path_builder: &mut PathBuilder,
    corner_x: f32,
    corner_y: f32,
    x: f32,
    y: f32,
) {
    path_builder.quad_to(
        corner_x + (x - corner_x) / 4.0,
        corner_y + (y - corner_y) / 4.0,
        x,
        y,
    );
}

#[derive(Debug, Clone)]
pub struct DrawEvent {
    top_left: Point,
    pixmap: Rc<RefCell<Pixmap>>,
    mask: Mask,
    transform: Transform,
    mask_rect: Rect,
}

fn relative_pos(rect: &Rect, offset: RelativeOffset) -> tiny_skia::Point {
    let x = rect.left().to_i32() as f32 + offset.x * rect.size_x().to_i32() as f32;
    let y = rect.top().to_i32() as f32 + offset.y * rect.size_y().to_i32() as f32;
    tiny_skia::Point::from_xy(x, y)
}

impl DrawEvent {
    pub fn new(pixmap: Rc<RefCell<Pixmap>>, top_left: Point, mask_rect: Rect) -> Self {
        let mut mask = Mask::new(pixmap.borrow().width(), pixmap.borrow().height()).unwrap();
        mask.fill_path(
            &PathBuilder::from_rect(
                tiny_skia::Rect::try_from(mask_rect).expect("invalid rect"), // TODO: handle error?
            ),
            FillRule::default(),
            false,
            Transform::default(),
        );

        Self {
            top_left,
            pixmap,
            mask,
            transform: Transform::from_translate(
                top_left.x().to_i32() as f32,
                top_left.y().to_i32() as f32,
            ),
            mask_rect,
        }
    }

    pub fn draw_pixmap(&self, pos: Point, pixmap: PixmapRef<'_>, transform: Transform) {
        self.pixmap.borrow_mut().draw_pixmap(
            pos.x().to_i32(),
            pos.y().to_i32(),
            pixmap,
            &PixmapPaint::default(),
            self.transform.pre_concat(transform),
            Some(&self.mask),
        )
    }

    pub fn draw_subpixmap(&self, target_rect: Rect, pixmap: PixmapRef<'_>, pixmap_offset: Point) {
        if target_rect.size_x() < 0.ppx() || target_rect.size_y() < 0.ppx() {
            warn!("negative size rect in draw_subpixmap");
            return;
        }
        if target_rect.is_empty() {
            return;
        }
        let translation = target_rect.top_left() - pixmap_offset;
        let patt_transform = Transform::from_translate(
            translation.x().to_i32() as f32,
            translation.y().to_i32() as f32,
        );
        let paint = Paint {
            shader: Pattern::new(
                pixmap,
                SpreadMode::Pad, // Pad, otherwise we will get weird borders overlap.
                FilterQuality::Nearest,
                1.0,
                patt_transform,
            ),
            blend_mode: BlendMode::default(),
            anti_alias: false,        // Skia doesn't use it too.
            force_hq_pipeline: false, // Pattern will use hq anyway.
        };

        self.pixmap.borrow_mut().fill_rect(
            tiny_skia::Rect::try_from(target_rect).unwrap(),
            &paint,
            self.transform,
            Some(&self.mask),
        );
    }

    fn rounded_rect_path(&self, rect: Rect, mut radius: f32, width: f32) -> Path {
        if radius > (rect.size_x().to_i32() as f32 / 2.0)
            || radius > (rect.size_y().to_i32() as f32 / 2.0)
        {
            //TODO do something here, log some error
            warn!("radius is bigger than fits in rectangle");
            radius = 0.0;
        }
        let top_left_point = rect.top_left();
        let top_left = tiny_skia::Point {
            x: top_left_point.x().to_i32() as f32 + width / 2.0,
            y: top_left_point.y().to_i32() as f32 + width / 2.0,
        };
        let size = tiny_skia::Point {
            x: rect.size_x().to_i32() as f32 - width,
            y: rect.size_y().to_i32() as f32 - width,
        };
        let mut path_builder = PathBuilder::new();
        path_builder.move_to(top_left.x + radius, top_left.y);
        path_builder.line_to(top_left.x + size.x - radius, top_left.y);
        rounded_line_in_square_corner(
            &mut path_builder,
            top_left.x + size.x,
            top_left.y,
            top_left.x + size.x,
            top_left.y + radius,
        );
        path_builder.line_to(top_left.x + size.x, top_left.y + size.y - radius);
        rounded_line_in_square_corner(
            &mut path_builder,
            top_left.x + size.x,
            top_left.y + size.y,
            top_left.x + size.x - radius,
            top_left.y + size.y,
        );
        path_builder.line_to(top_left.x + radius, top_left.y + size.y);
        rounded_line_in_square_corner(
            &mut path_builder,
            top_left.x,
            top_left.y + size.y,
            top_left.x,
            top_left.y + size.y - radius,
        );
        path_builder.line_to(top_left.x, top_left.y + radius);
        rounded_line_in_square_corner(
            &mut path_builder,
            top_left.x,
            top_left.y,
            top_left.x + radius,
            top_left.y,
        );
        path_builder.finish().unwrap()
    }

    // TODO: translate to current rect
    pub fn stroke_path(&self, path: &Path, color: Color, width: f32) {
        self.pixmap.borrow_mut().stroke_path(
            path,
            &Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Paint::default()
            },
            &Stroke {
                width,
                ..Stroke::default()
            },
            self.transform,
            Some(&self.mask),
        );
    }

    // TODO: translate to current rect
    pub fn fill_path(&self, path: &Path, shader: Shader) {
        self.pixmap.borrow_mut().fill_path(
            path,
            &Paint {
                shader,
                ..Paint::default()
            },
            FillRule::default(),
            self.transform,
            Some(&self.mask),
        );
    }

    pub fn stroke_and_fill_rounded_rect(
        &self,
        rect: Rect,
        border: &ComputedBorderStyle,
        background: Option<&ComputedBackground>,
    ) {
        let path = self.rounded_rect_path(
            rect,
            border.radius.to_i32() as f32,
            border.width.to_i32() as f32,
        );
        if let Some(background) = background {
            let shader = match background {
                ComputedBackground::Solid { color } => Shader::SolidColor(*color),
                ComputedBackground::LinearGradient(gradient) => LinearGradient::new(
                    relative_pos(&rect, gradient.start),
                    relative_pos(&rect, gradient.end),
                    gradient.stops.clone(),
                    gradient.mode,
                    Transform::default(),
                )
                .unwrap_or_else(|| {
                    warn!("failed to create gradient");
                    Shader::SolidColor(Color::TRANSPARENT)
                }),
            };
            self.fill_path(&path, shader);
        }
        if border.width > 0.ppx() {
            self.stroke_path(&path, border.color, border.width.to_i32() as f32);
        }
    }

    pub fn fill_rounded_rect(&self, rect: Rect, radius: f32, width: f32, shader: Shader) {
        let path = self.rounded_rect_path(rect, radius, width);
        self.fill_path(&path, shader);
    }

    // TODO: PhysicalPixels?
    pub fn stroke_rounded_rect(&self, rect: Rect, radius: f32, color: Color, width: f32) {
        if width == 0.0 {
            return;
        }
        let path = self.rounded_rect_path(rect, radius, width);
        self.stroke_path(&path, color, width);
    }

    // TODO: add at least width
    pub fn stroke_rect(&self, rect: Rect, color: Color) {
        let path = PathBuilder::from_rect(tiny_skia::Rect::try_from(rect).unwrap());
        self.pixmap.borrow_mut().stroke_path(
            &path,
            &Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Paint::default()
            },
            &Stroke::default(),
            self.transform,
            Some(&self.mask),
        );
    }

    pub fn fill_rect(&self, rect: Rect, color: Color) {
        self.pixmap.borrow_mut().fill_rect(
            tiny_skia::Rect::try_from(rect).unwrap(),
            &Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Paint::default()
            },
            self.transform,
            Some(&self.mask),
        );
    }

    pub fn map_to_child(&self, rect_in_parent: Rect) -> Option<Self> {
        let rect = rect_in_parent.translate(self.top_left);
        let mask_rect = self.mask_rect.intersect(rect);
        if mask_rect.is_empty() {
            return None;
        }

        Some(Self::new(
            Rc::clone(&self.pixmap),
            rect.top_left(),
            mask_rect,
        ))
    }
}
