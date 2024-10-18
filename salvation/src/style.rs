use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{anyhow, bail, Context, Result};
use lightningcss::{
    properties::Property, rules::CssRule, selector::Selector, stylesheet::StyleSheet,
};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use tiny_skia::Pixmap;
use usvg::TreeParsing;

use crate::{
    style::css::replace_vars,
    types::{LogicalPixels, Point},
};

pub mod button;
pub mod computed;
pub mod css;
pub mod defaults;
pub mod grid;
pub mod image;
pub mod scroll_bar;
pub mod text_input;

pub trait ElementState: Eq + Hash + Sized {
    fn all() -> Vec<Self>;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RelativeOffset {
    // from 0 to 1
    pub x: f32,
    pub y: f32,
}

impl RelativeOffset {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Padding {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

impl Padding {
    pub fn new(x: LogicalPixels, y: LogicalPixels) -> Self {
        Self { x, y }
    }
    pub fn to_physical(self, scale: f32) -> Point {
        Point {
            x: self.x.to_physical(scale).get(),
            y: self.y.to_physical(scale).get(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontStyle {
    pub font_size: LogicalPixels,
    pub line_height: LogicalPixels,
    // TODO: font family, attributes, etc.
}

impl FontStyle {
    pub fn to_metrics(&self, scale: f32) -> salvation_cosmic_text::Metrics {
        salvation_cosmic_text::Metrics {
            font_size: self.font_size.to_physical(scale).get() as f32,
            line_height: self.line_height.to_physical(scale).get() as f32,
        }
    }
}

// TODO: not pub
#[derive(Debug)]
pub enum StyleSource {
    File {
        parent_dir: PathBuf,
    },
    Bundle {
        files: HashMap<&'static str, &'static [u8]>,
    },
}

#[derive(Debug)]
pub struct Style {
    pub css: StyleSheet<'static, 'static>,
    pub source: StyleSource,
}

fn load_css(css: &str) -> Result<StyleSheet<'static, 'static>> {
    let mut style = StyleSheet::parse(css, Default::default())
        .map_err(|e| anyhow!("failed to parse css: {e}"))?;
    replace_vars(&mut style);
    let code = style.to_css(Default::default())?.code;
    let style = StyleSheet::parse(&code, Default::default())
        .map_err(|e| anyhow!("failed to parse css: {e}"))?;
    // There is no StyleSheet::into_owned, so we have to use serialization :(
    let serialized = serde_value::to_value(&style)?;

    // println!("{style:#?}");
    Ok(serialized.deserialize_into()?)
}

impl Style {
    pub fn load_bundled(
        css: &str,
        files: impl IntoIterator<Item = (&'static str, &'static [u8])>,
    ) -> Result<Self> {
        Ok(Self {
            css: load_css(css)?,
            source: StyleSource::Bundle {
                files: files.into_iter().collect(),
            },
        })
    }

    pub fn load_from_file(css_path: &Path) -> Result<Style> {
        let css = fs_err::read_to_string(css_path)?;

        Ok(Self {
            css: load_css(&css)?,
            source: StyleSource::File {
                parent_dir: css_path
                    .parent()
                    .context("invalid css path (couldn't get parent)")?
                    .into(),
            },
        })
    }

    pub fn find_rules(
        &self,
        check_selector: impl Fn(&Selector) -> bool,
    ) -> Vec<&Property<'static>> {
        let mut results = Vec::new();
        for rule in &self.css.rules.0 {
            if let CssRule::Style(rule) = rule {
                for selector in &rule.selectors.0 {
                    if check_selector(selector) {
                        let specificity = selector.specificity();
                        results.extend(
                            rule.declarations
                                .iter()
                                .map(|(dec, important)| (important, specificity, dec)),
                        );
                    }
                }
            }
        }
        // Use stable sort because later statements should take priority.
        results.sort_by_key(|(important, specificity, _dec)| (*important, *specificity));
        results.into_iter().map(|(_, _, dec)| dec).collect()
    }

    pub fn load_resource(&self, path: &str) -> Result<Cow<'static, [u8]>> {
        match &self.source {
            // TODO: forbid "../", allow only simple paths
            StyleSource::File { parent_dir } => {
                let path = parent_dir.join(path);
                Ok(Cow::Owned(fs_err::read(path)?))
            }
            StyleSource::Bundle { files } => {
                Ok(Cow::Borrowed(files.get(path).with_context(|| {
                    format!("no such file in bundle: {path:?}")
                })?))
            }
        }
    }

    pub fn load_pixmap(&self, path: &str, scale: f32) -> Result<Rc<Pixmap>> {
        // TODO: cache pixmaps
        // TODO: support png
        if !path.ends_with(".svg") {
            bail!("only svg is currently supported");
        }
        let data = self.load_resource(path)?;

        let tree = usvg::Tree::from_data(&data, &Default::default())?;
        let rtree = resvg::Tree::from_usvg(&tree);

        let pixmap_size_x = (rtree.size.width() * scale).ceil() as u32;
        let pixmap_size_y = (rtree.size.height() * scale).ceil() as u32;
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size_x, pixmap_size_y).unwrap();
        rtree.render(
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        Ok(Rc::new(pixmap))
    }
}
