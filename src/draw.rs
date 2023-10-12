use std::{cell::RefCell, rc::Rc};

use log::warn;
use tiny_skia::{
    BlendMode, Color, FillRule, FilterQuality, LinearGradient, Mask, Paint, Path, PathBuilder,
    Pattern, Pixmap, PixmapPaint, PixmapRef, Shader, SpreadMode, Stroke, Transform,
};

use crate::{
    style::{computed::ComputedBorderStyle, Background},
    types::{Point, Rect},
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
    pixmap: Rc<RefCell<Pixmap>>,
    mask: Mask,
    top_left: Point,
    mask_rect: Rect,
}

impl DrawEvent {
    pub fn new(pixmap: Rc<RefCell<Pixmap>>, top_left: Point, mask_rect: Rect) -> Self {
        let mut mask = Mask::new(pixmap.borrow().width(), pixmap.borrow().height()).unwrap();
        mask.fill_path(
            &PathBuilder::from_rect(
                tiny_skia::Rect::from_xywh(
                    mask_rect.top_left.x as f32,
                    mask_rect.top_left.y as f32,
                    mask_rect.size.x as f32,
                    mask_rect.size.y as f32,
                )
                .unwrap(),
            ),
            FillRule::default(),
            false,
            Transform::default(),
        );

        Self {
            pixmap,
            mask,
            top_left,
            mask_rect,
        }
    }

    pub fn draw_pixmap(&self, pos: Point, pixmap: PixmapRef<'_>) {
        self.pixmap.borrow_mut().draw_pixmap(
            pos.x + self.top_left.x,
            pos.y + self.top_left.y,
            pixmap,
            &PixmapPaint::default(),
            Transform::default(),
            Some(&self.mask),
        )
    }

    pub fn draw_subpixmap(&self, target_rect: Rect, pixmap: PixmapRef<'_>, pixmap_offset: Point) {
        if target_rect.size.x < 0 || target_rect.size.y < 0 {
            warn!("negative size rect in draw_subpixmap");
            return;
        }
        if target_rect.is_empty() {
            return;
        }
        let target_rect = target_rect.translate(self.top_left);
        let translation = target_rect.top_left - pixmap_offset;
        let patt_transform = Transform::from_translate(translation.x as f32, translation.y as f32);
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
            tiny_skia::Rect::from_xywh(
                target_rect.top_left.x as f32,
                target_rect.top_left.y as f32,
                target_rect.size.x as f32,
                target_rect.size.y as f32,
            )
            .unwrap(),
            &paint,
            Transform::default(),
            Some(&self.mask),
        );
    }

    fn rounded_rect_path(&self, rect: Rect, mut radius: f32, width: f32) -> Path {
        if radius > (rect.size.x as f32 / 2.0) || radius > (rect.size.y as f32 / 2.0) {
            //TODO do something here, log some error
            warn!("radius is bigger than fits in rectangle");
            radius = 0.0;
        }
        let top_left_point = self.top_left + rect.top_left;
        let top_left = tiny_skia::Point {
            x: top_left_point.x as f32 + width / 2.0,
            y: top_left_point.y as f32 + width / 2.0,
        };
        let size = tiny_skia::Point {
            x: rect.size.x as f32 - width,
            y: rect.size.y as f32 - width,
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
            Transform::default(),
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
            Transform::default(),
            Some(&self.mask),
        );
    }

    pub fn stroke_and_fill_rounded_rect(
        &self,
        rect: Rect,
        border: Option<&ComputedBorderStyle>,
        background: Option<&Background>,
    ) {
        let path = self.rounded_rect_path(
            rect,
            border.map_or(0.0, |b| b.radius.get() as f32),
            border.map_or(0.0, |b| b.width.get() as f32),
        );
        if let Some(background) = background {
            let global_rect = rect.translate(self.top_left);
            let shader = match background {
                Background::Solid(color) => Shader::SolidColor((*color).into()),
                Background::LinearGradient(gradient) => LinearGradient::new(
                    global_rect.relative_pos(gradient.start).into(),
                    global_rect.relative_pos(gradient.end).into(),
                    // TODO: computed background?
                    gradient
                        .stops
                        .iter()
                        .map(|stop| stop.clone().into())
                        .collect(),
                    gradient.mode.into(),
                    Transform::default(),
                )
                .unwrap_or_else(|| {
                    warn!("failed to create gradient");
                    Shader::SolidColor(Color::TRANSPARENT)
                }),
            };
            self.fill_path(&path, shader);
        }
        if let Some(border) = border {
            self.stroke_path(&path, border.color, border.width.get() as f32);
        }
    }

    pub fn fill_rounded_rect(&self, rect: Rect, radius: f32, width: f32, shader: Shader) {
        let path = self.rounded_rect_path(rect, radius, width);
        self.fill_path(&path, shader);
    }

    // TODO: PhysicalPixels?
    pub fn stroke_rounded_rect(&self, rect: Rect, radius: f32, color: Color, width: f32) {
        let path = self.rounded_rect_path(rect, radius, width);
        self.stroke_path(&path, color, width);
    }

    // TODO: add at least width
    pub fn stroke_rect(&self, rect: Rect, color: Color) {
        let top_left = self.top_left + rect.top_left;
        let path = PathBuilder::from_rect(
            tiny_skia::Rect::from_xywh(
                top_left.x as f32 + 0.5,
                top_left.y as f32 + 0.5,
                rect.size.x as f32 - 1.0,
                rect.size.y as f32 - 1.0,
            )
            .unwrap(),
        );
        self.pixmap.borrow_mut().stroke_path(
            &path,
            &Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Paint::default()
            },
            &Stroke::default(),
            Transform::default(),
            Some(&self.mask),
        );
    }

    pub fn fill_rect(&self, rect: Rect, color: Color) {
        let top_left = self.top_left + rect.top_left;
        self.pixmap.borrow_mut().fill_rect(
            tiny_skia::Rect::from_xywh(
                top_left.x as f32,
                top_left.y as f32,
                rect.size.x as f32,
                rect.size.y as f32,
            )
            .unwrap(),
            &Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Paint::default()
            },
            Transform::default(),
            Some(&self.mask),
        );
    }

    pub fn map_to_child(&self, rect_in_parent: Rect) -> Option<Self> {
        let rect = rect_in_parent.translate(self.top_left);
        let mask_rect = self.mask_rect.intersect(rect);
        if mask_rect.is_empty() {
            return None;
        }

        Some(Self::new(Rc::clone(&self.pixmap), rect.top_left, mask_rect))
    }
}
