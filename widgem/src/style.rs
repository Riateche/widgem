use {
    crate::{
        style::{
            common::ComputedElementStyle,
            css::{
                convert_background_color, convert_font, convert_main_color, is_root, replace_vars,
                StyleSelector,
            },
        },
        types::{LogicalPixels, Point},
        Pixmap,
    },
    anyhow::{anyhow, bail, Context, Result},
    lightningcss::{
        properties::Property, rules::CssRule, selector::Selector, stylesheet::StyleSheet,
    },
    log::warn,
    ordered_float::OrderedFloat,
    serde::{Deserialize, Serialize},
    std::{
        any::{Any, TypeId},
        borrow::Cow,
        collections::HashMap,
        fmt::Debug,
        hash::Hash,
        path::{Path, PathBuf},
        rc::Rc,
    },
    tiny_skia::Color,
};

pub mod common;
pub mod css;
pub mod defaults;
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
        Point::new(self.x.to_physical(scale), self.y.to_physical(scale))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontStyle {
    pub font_size: LogicalPixels,
    pub line_height: LogicalPixels,
    // TODO: font family, attributes, etc.
}

impl FontStyle {
    pub fn to_metrics(&self, scale: f32) -> cosmic_text::Metrics {
        cosmic_text::Metrics {
            font_size: self.font_size.to_physical(scale).to_i32() as f32,
            line_height: self.line_height.to_physical(scale).to_i32() as f32,
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

    cache: HashMap<(StyleSelector, OrderedFloat<f32>, TypeId), Box<dyn Any>>,
}

fn find_rules<'a>(
    style: &'a StyleSheet<'static, 'static>,
    check_selector: impl Fn(&Selector<'static>) -> bool,
) -> Vec<&'a Property<'static>> {
    let mut results = Vec::new();
    for rule in &style.rules.0 {
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

pub(crate) fn load_css(css: &str) -> Result<StyleSheet<'static, 'static>> {
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
            cache: HashMap::new(),
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
            cache: HashMap::new(),
        })
    }

    pub fn find_rules_for_element(&self, element: &StyleSelector) -> Vec<&Property<'static>> {
        self.find_rules(|selector| element.matches(selector))
    }

    pub fn find_rules(
        &self,
        check_selector: impl Fn(&Selector<'static>) -> bool,
    ) -> Vec<&Property<'static>> {
        find_rules(&self.css, check_selector)
    }

    // TODO: cache?
    pub fn root_font_style(&self) -> FontStyle {
        let rules = self.find_rules(is_root);
        convert_font(&rules, None)
    }

    pub fn root_background_color(&self) -> Color {
        let rules = self.find_rules(is_root);
        convert_background_color(&rules).unwrap_or_else(defaults::background_color)
    }

    pub fn root_color(&self) -> Color {
        let rules = self.find_rules(is_root);
        convert_main_color(&rules).unwrap_or_else(|| {
            warn!("missing 'color' property for :root in style");
            defaults::text_color()
        })
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

    pub fn load_pixmap(&self, path: &str, scale: f32) -> Result<Pixmap> {
        // TODO: cache pixmaps
        // TODO: support png
        if !path.ends_with(".svg") {
            bail!("only svg is currently supported");
        }
        let data = self.load_resource(path)?;

        let tree = usvg::Tree::from_data(&data, &Default::default())?;

        let pixmap_size_x = (tree.size().width() * scale).ceil() as u32;
        let pixmap_size_y = (tree.size().height() * scale).ceil() as u32;
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size_x, pixmap_size_y).unwrap();
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        Ok(pixmap.into())
    }

    pub fn get<T: ComputedElementStyle>(
        &mut self,
        element: &StyleSelector,
        scale: f32,
        custom_style: Option<&StyleSheet<'static, 'static>>,
    ) -> Rc<T> {
        if let Some(custom_style) = custom_style {
            let styles = Styles {
                main: self,
                custom: Some(custom_style),
            };
            // TODO: avoid unneeded Rc
            return Rc::new(T::new(&styles, element, scale));
        }
        let type_id = TypeId::of::<T>();
        let key = (element.clone(), OrderedFloat(scale), type_id);
        if let Some(data) = self.cache.get(&key) {
            return data
                .downcast_ref::<Rc<T>>()
                .expect("style cache type mismatch")
                .clone();
        }
        let style = Rc::new(T::new(
            &Styles {
                main: self,
                custom: None,
            },
            element,
            scale,
        ));
        let style_clone = style.clone();
        self.cache.insert(key, Box::new(style));
        style_clone
    }
}

#[derive(Debug)]
pub struct Styles<'a> {
    main: &'a Style,
    custom: Option<&'a StyleSheet<'static, 'static>>,
}

impl<'a> Styles<'a> {
    pub fn find_rules_for_element(&self, element: &StyleSelector) -> Vec<&Property<'static>> {
        self.find_rules(|selector| element.matches(selector))
    }

    pub fn find_rules(
        &self,
        check_selector: impl Fn(&Selector<'static>) -> bool,
    ) -> Vec<&Property<'static>> {
        let mut rules = self.main.find_rules(&check_selector);
        if let Some(custom) = self.custom {
            rules.extend(find_rules(custom, check_selector));
        }
        rules
    }

    // TODO: cache?
    pub fn root_font_style(&self) -> FontStyle {
        self.main.root_font_style()
    }

    pub fn root_color(&self) -> Color {
        self.main.root_color()
    }

    pub fn load_pixmap(&self, path: &str, scale: f32) -> Result<Pixmap> {
        self.main.load_pixmap(path, scale)
    }
}
