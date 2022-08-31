//! CSS properties related to borders.

use super::border_image::*;
use super::border_radius::*;
use crate::compat::Feature;
use crate::context::PropertyHandlerContext;
use crate::declaration::{DeclarationBlock, DeclarationList};
use crate::error::{ParserError, PrinterError};
use crate::logical::PropertyCategory;
use crate::macros::*;
use crate::printer::Printer;
use crate::properties::custom::UnparsedProperty;
use crate::properties::{Property, PropertyId};
use crate::targets::Browsers;
use crate::traits::{FallbackValues, Parse, PropertyHandler, Shorthand, ToCss};
use crate::values::color::{ColorFallbackKind, CssColor};
use crate::values::length::*;
use crate::values::rect::Rect;
use crate::values::size::Size2D;
use cssparser::*;

/// A value for the [border-width](https://www.w3.org/TR/css-backgrounds-3/#border-width) property.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
pub enum BorderSideWidth {
  /// A UA defined `thin` value.
  Thin,
  /// A UA defined `medium` value.
  Medium,
  /// A UA defined `thick` value.
  Thick,
  /// An explicit width.
  Length(Length),
}

impl Default for BorderSideWidth {
  fn default() -> BorderSideWidth {
    BorderSideWidth::Medium
  }
}

impl<'i> Parse<'i> for BorderSideWidth {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if let Ok(length) = input.try_parse(|i| Length::parse(i)) {
      return Ok(BorderSideWidth::Length(length));
    }
    let location = input.current_source_location();
    let ident = input.expect_ident()?;
    match_ignore_ascii_case! { &ident,
      "thin" => Ok(BorderSideWidth::Thin),
      "medium" => Ok(BorderSideWidth::Medium),
      "thick" => Ok(BorderSideWidth::Thick),
      _ => return Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
    }
  }
}

impl ToCss for BorderSideWidth {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    use BorderSideWidth::*;
    match self {
      Thin => dest.write_str("thin"),
      Medium => dest.write_str("medium"),
      Thick => dest.write_str("thick"),
      Length(length) => length.to_css(dest),
    }
  }
}

enum_property! {
  /// A [`<line-style>`](https://drafts.csswg.org/css-backgrounds/#typedef-line-style) value, used in the `border-style` property.
  pub enum LineStyle {
    /// No border.
    None,
    /// Similar to `none` but with different rules for tables.
    Hidden,
    /// Looks as if the content on the inside of the border is sunken into the canvas.
    Inset,
    /// Looks as if it were carved in the canvas.
    Groove,
    /// Looks as if the content on the inside of the border is coming out of the canvas.
    Outset,
    /// Looks as if it were coming out of the canvas.
    Ridge,
    /// A series of round dots.
    Dotted,
    /// A series of square-ended dashes.
    Dashed,
    /// A single line segment.
    Solid,
    /// Two parallel solid lines with some space between them.
    Double,
  }
}

impl Default for LineStyle {
  fn default() -> LineStyle {
    LineStyle::None
  }
}

/// A generic type that represents the `border` and `outline` shorthand properties.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GenericBorder<S, const P: u8> {
  /// The width of the border.
  pub width: BorderSideWidth,
  /// The border style.
  pub style: S,
  /// The border color.
  pub color: CssColor,
}

impl<S: Default, const P: u8> Default for GenericBorder<S, P> {
  fn default() -> GenericBorder<S, P> {
    GenericBorder {
      width: BorderSideWidth::Medium,
      style: S::default(),
      color: CssColor::current_color(),
    }
  }
}

impl<'i, S: Parse<'i> + Default, const P: u8> Parse<'i> for GenericBorder<S, P> {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    // Order doesn't matter...
    let mut color = None;
    let mut style = None;
    let mut width = None;
    let mut any = false;
    loop {
      if width.is_none() {
        if let Ok(value) = input.try_parse(|i| BorderSideWidth::parse(i)) {
          width = Some(value);
          any = true;
        }
      }
      if style.is_none() {
        if let Ok(value) = input.try_parse(S::parse) {
          style = Some(value);
          any = true;
          continue;
        }
      }
      if color.is_none() {
        if let Ok(value) = input.try_parse(|i| CssColor::parse(i)) {
          color = Some(value);
          any = true;
          continue;
        }
      }
      break;
    }
    if any {
      Ok(GenericBorder {
        width: width.unwrap_or(BorderSideWidth::Medium),
        style: style.unwrap_or_default(),
        color: color.unwrap_or_else(|| CssColor::current_color()),
      })
    } else {
      Err(input.new_custom_error(ParserError::InvalidDeclaration))
    }
  }
}

impl<S: ToCss + Default + PartialEq, const P: u8> ToCss for GenericBorder<S, P> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    if *self == Self::default() {
      self.style.to_css(dest)?;
      return Ok(());
    }

    let mut needs_space = false;
    if self.width != BorderSideWidth::default() {
      self.width.to_css(dest)?;
      needs_space = true;
    }
    if self.style != S::default() {
      if needs_space {
        dest.write_str(" ")?;
      }
      self.style.to_css(dest)?;
      needs_space = true;
    }
    if self.color != CssColor::current_color() {
      if needs_space {
        dest.write_str(" ")?;
      }
      self.color.to_css(dest)?;
    }
    Ok(())
  }
}

impl<S: Clone, const P: u8> FallbackValues for GenericBorder<S, P> {
  fn get_fallbacks(&mut self, targets: Browsers) -> Vec<Self> {
    self
      .color
      .get_fallbacks(targets)
      .into_iter()
      .map(|color| GenericBorder {
        color,
        width: self.width.clone(),
        style: self.style.clone(),
      })
      .collect()
  }
}

/// A value for the [border-top](https://www.w3.org/TR/css-backgrounds-3/#propdef-border-top) shorthand property.
pub type BorderTop = GenericBorder<LineStyle, 0>;
/// A value for the [border-right](https://www.w3.org/TR/css-backgrounds-3/#propdef-border-right) shorthand property.
pub type BorderRight = GenericBorder<LineStyle, 1>;
/// A value for the [border-bottom](https://www.w3.org/TR/css-backgrounds-3/#propdef-border-bottom) shorthand property.
pub type BorderBottom = GenericBorder<LineStyle, 2>;
/// A value for the [border-left](https://www.w3.org/TR/css-backgrounds-3/#propdef-border-left) shorthand property.
pub type BorderLeft = GenericBorder<LineStyle, 3>;
/// A value for the [border-block-start](https://drafts.csswg.org/css-logical/#propdef-border-block-start) shorthand property.
pub type BorderBlockStart = GenericBorder<LineStyle, 4>;
/// A value for the [border-block-end](https://drafts.csswg.org/css-logical/#propdef-border-block-end) shorthand property.
pub type BorderBlockEnd = GenericBorder<LineStyle, 5>;
/// A value for the [border-inline-start](https://drafts.csswg.org/css-logical/#propdef-border-inline-start) shorthand property.
pub type BorderInlineStart = GenericBorder<LineStyle, 6>;
/// A value for the [border-inline-end](https://drafts.csswg.org/css-logical/#propdef-border-inline-end) shorthand property.
pub type BorderInlineEnd = GenericBorder<LineStyle, 7>;
/// A value for the [border-block](https://drafts.csswg.org/css-logical/#propdef-border-block) shorthand property.
pub type BorderBlock = GenericBorder<LineStyle, 8>;
/// A value for the [border-inline](https://drafts.csswg.org/css-logical/#propdef-border-inline) shorthand property.
pub type BorderInline = GenericBorder<LineStyle, 9>;
/// A value for the [border](https://www.w3.org/TR/css-backgrounds-3/#propdef-border) shorthand property.
pub type Border = GenericBorder<LineStyle, 10>;

impl_shorthand! {
  BorderTop(BorderTop) {
    width: [BorderTopWidth],
    style: [BorderTopStyle],
    color: [BorderTopColor],
  }
}

impl_shorthand! {
  BorderRight(BorderRight) {
    width: [BorderRightWidth],
    style: [BorderRightStyle],
    color: [BorderRightColor],
  }
}

impl_shorthand! {
  BorderBottom(BorderBottom) {
    width: [BorderBottomWidth],
    style: [BorderBottomStyle],
    color: [BorderBottomColor],
  }
}

impl_shorthand! {
  BorderLeft(BorderLeft) {
    width: [BorderLeftWidth],
    style: [BorderLeftStyle],
    color: [BorderLeftColor],
  }
}

impl_shorthand! {
  BorderBlockStart(BorderBlockStart) {
    width: [BorderBlockStartWidth],
    style: [BorderBlockStartStyle],
    color: [BorderBlockStartColor],
  }
}

impl_shorthand! {
  BorderBlockEnd(BorderBlockEnd) {
    width: [BorderBlockEndWidth],
    style: [BorderBlockEndStyle],
    color: [BorderBlockEndColor],
  }
}

impl_shorthand! {
  BorderInlineStart(BorderInlineStart) {
    width: [BorderInlineStartWidth],
    style: [BorderInlineStartStyle],
    color: [BorderInlineStartColor],
  }
}

impl_shorthand! {
  BorderInlineEnd(BorderInlineEnd) {
    width: [BorderInlineEndWidth],
    style: [BorderInlineEndStyle],
    color: [BorderInlineEndColor],
  }
}

impl_shorthand! {
  BorderBlock(BorderBlock) {
    width: [BorderBlockStartWidth, BorderBlockEndWidth],
    style: [BorderBlockStartStyle, BorderBlockEndStyle],
    color: [BorderBlockStartColor, BorderBlockEndColor],
  }
}

impl_shorthand! {
  BorderInline(BorderInline) {
    width: [BorderInlineStartWidth, BorderInlineEndWidth],
    style: [BorderInlineStartStyle, BorderInlineEndStyle],
    color: [BorderInlineStartColor, BorderInlineEndColor],
  }
}

impl_shorthand! {
  Border(Border) {
    width: [BorderTopWidth, BorderRightWidth, BorderBottomWidth, BorderLeftWidth],
    style: [BorderTopStyle, BorderRightStyle, BorderBottomStyle, BorderLeftStyle],
    color: [BorderTopColor, BorderRightColor, BorderBottomColor, BorderLeftColor],
  }
}

size_shorthand! {
  /// A value for the [border-block-color](https://drafts.csswg.org/css-logical/#propdef-border-block-color) shorthand property.
  pub struct BorderBlockColor<CssColor> {
    /// The block start value.
    start: BorderBlockStartColor,
    /// The block end value.
    end: BorderBlockEndColor,
  }
}

size_shorthand! {
  /// A value for the [border-block-style](https://drafts.csswg.org/css-logical/#propdef-border-block-style) shorthand property.
  pub struct BorderBlockStyle<LineStyle> {
    /// The block start value.
    start: BorderBlockStartStyle,
    /// The block end value.
    end: BorderBlockEndStyle,
  }
}

size_shorthand! {
  /// A value for the [border-block-width](https://drafts.csswg.org/css-logical/#propdef-border-block-width) shorthand property.
  pub struct BorderBlockWidth<BorderSideWidth> {
    /// The block start value.
    start: BorderBlockStartWidth,
    /// The block end value.
    end: BorderBlockEndWidth,
  }
}

size_shorthand! {
  /// A value for the [border-inline-color](https://drafts.csswg.org/css-logical/#propdef-border-inline-color) shorthand property.
  pub struct BorderInlineColor<CssColor> {
    /// The inline start value.
    start: BorderInlineStartColor,
    /// The inline end value.
    end: BorderInlineEndColor,
  }
}

size_shorthand! {
  /// A value for the [border-inline-style](https://drafts.csswg.org/css-logical/#propdef-border-inline-style) shorthand property.
  pub struct BorderInlineStyle<LineStyle> {
    /// The inline start value.
    start: BorderInlineStartStyle,
    /// The inline end value.
    end: BorderInlineEndStyle,
  }
}

size_shorthand! {
  /// A value for the [border-inline-width](https://drafts.csswg.org/css-logical/#propdef-border-inline-width) shorthand property.
  pub struct BorderInlineWidth<BorderSideWidth> {
    /// The inline start value.
    start: BorderInlineStartWidth,
    /// The inline end value.
    end: BorderInlineEndWidth,
  }
}

rect_shorthand! {
  /// A value for the [border-color](https://drafts.csswg.org/css-backgrounds/#propdef-border-color) shorthand property.
  pub struct BorderColor<CssColor> {
    BorderTopColor,
    BorderRightColor,
    BorderBottomColor,
    BorderLeftColor
  }
}

rect_shorthand! {
  /// A value for the [border-style](https://drafts.csswg.org/css-backgrounds/#propdef-border-style) shorthand property.
  pub struct BorderStyle<LineStyle> {
    BorderTopStyle,
    BorderRightStyle,
    BorderBottomStyle,
    BorderLeftStyle
  }
}

rect_shorthand! {
  /// A value for the [border-width](https://drafts.csswg.org/css-backgrounds/#propdef-border-width) shorthand property.
  pub struct BorderWidth<BorderSideWidth> {
    BorderTopWidth,
    BorderRightWidth,
    BorderBottomWidth,
    BorderLeftWidth
  }
}

macro_rules! impl_fallbacks {
  ($t: ident $(, $name: ident)+) => {
    impl FallbackValues for $t {
      fn get_fallbacks(&mut self, targets: Browsers) -> Vec<Self> {
        let mut fallbacks = ColorFallbackKind::empty();
        $(
          fallbacks |= self.$name.get_necessary_fallbacks(targets);
        )+

        let mut res = Vec::new();
        if fallbacks.contains(ColorFallbackKind::RGB) {
          res.push($t {
            $(
              $name: self.$name.get_fallback(ColorFallbackKind::RGB),
            )+
          });
        }

        if fallbacks.contains(ColorFallbackKind::P3) {
          res.push($t {
            $(
              $name: self.$name.get_fallback(ColorFallbackKind::P3),
            )+
          });
        }

        if fallbacks.contains(ColorFallbackKind::LAB) {
          $(
            self.$name = self.$name.get_fallback(ColorFallbackKind::LAB);
          )+
        }

        res
      }
    }
  }
}

impl_fallbacks!(BorderBlockColor, start, end);
impl_fallbacks!(BorderInlineColor, start, end);
impl_fallbacks!(BorderColor, top, right, bottom, left);

#[derive(Default, Debug, PartialEq)]
struct BorderShorthand {
  pub width: Option<BorderSideWidth>,
  pub style: Option<LineStyle>,
  pub color: Option<CssColor>,
}

impl BorderShorthand {
  pub fn set_border<const P: u8>(&mut self, border: &GenericBorder<LineStyle, P>) {
    self.width = Some(border.width.clone());
    self.style = Some(border.style.clone());
    self.color = Some(border.color.clone());
  }

  pub fn is_valid(&self) -> bool {
    self.width.is_some() && self.style.is_some() && self.color.is_some()
  }

  pub fn reset(&mut self) {
    self.width = None;
    self.style = None;
    self.color = None;
  }

  pub fn to_border<const P: u8>(&self) -> GenericBorder<LineStyle, P> {
    GenericBorder {
      width: self.width.clone().unwrap(),
      style: self.style.clone().unwrap(),
      color: self.color.clone().unwrap(),
    }
  }
}

#[derive(Debug)]
pub(crate) struct BorderHandler<'i> {
  targets: Option<Browsers>,
  border_top: BorderShorthand,
  border_bottom: BorderShorthand,
  border_left: BorderShorthand,
  border_right: BorderShorthand,
  border_block_start: BorderShorthand,
  border_block_end: BorderShorthand,
  border_inline_start: BorderShorthand,
  border_inline_end: BorderShorthand,
  category: PropertyCategory,
  border_image_handler: BorderImageHandler<'i>,
  border_radius_handler: BorderRadiusHandler<'i>,
  has_any: bool,
}

impl<'i> BorderHandler<'i> {
  pub fn new(targets: Option<Browsers>) -> Self {
    BorderHandler {
      targets,
      border_top: BorderShorthand::default(),
      border_bottom: BorderShorthand::default(),
      border_left: BorderShorthand::default(),
      border_right: BorderShorthand::default(),
      border_block_start: BorderShorthand::default(),
      border_block_end: BorderShorthand::default(),
      border_inline_start: BorderShorthand::default(),
      border_inline_end: BorderShorthand::default(),
      category: PropertyCategory::default(),
      border_image_handler: BorderImageHandler::new(targets),
      border_radius_handler: BorderRadiusHandler::new(targets),
      has_any: false,
    }
  }
}

impl<'i> PropertyHandler<'i> for BorderHandler<'i> {
  fn handle_property(
    &mut self,
    property: &Property<'i>,
    dest: &mut DeclarationList<'i>,
    context: &mut PropertyHandlerContext<'i, '_>,
  ) -> bool {
    use Property::*;

    macro_rules! property {
      ($key: ident, $prop: ident, $val: expr, $category: ident) => {{
        if PropertyCategory::$category != self.category {
          self.flush(dest, context);
        }
        self.$key.$prop = Some($val.clone());
        self.category = PropertyCategory::$category;
        self.has_any = true;
      }};
    }

    macro_rules! set_border {
      ($key: ident, $val: ident, $category: ident) => {{
        if PropertyCategory::$category != self.category {
          self.flush(dest, context);
        }
        self.$key.set_border($val);
        self.category = PropertyCategory::$category;
        self.has_any = true;
      }};
    }

    match &property {
      BorderTopColor(val) => property!(border_top, color, val, Physical),
      BorderBottomColor(val) => property!(border_bottom, color, val, Physical),
      BorderLeftColor(val) => property!(border_left, color, val, Physical),
      BorderRightColor(val) => property!(border_right, color, val, Physical),
      BorderBlockStartColor(val) => property!(border_block_start, color, val, Logical),
      BorderBlockEndColor(val) => property!(border_block_end, color, val, Logical),
      BorderBlockColor(val) => {
        property!(border_block_start, color, val.start, Logical);
        property!(border_block_end, color, val.end, Logical);
      }
      BorderInlineStartColor(val) => property!(border_inline_start, color, val, Logical),
      BorderInlineEndColor(val) => property!(border_inline_end, color, val, Logical),
      BorderInlineColor(val) => {
        property!(border_inline_start, color, val.start, Logical);
        property!(border_inline_end, color, val.end, Logical);
      }
      BorderTopWidth(val) => property!(border_top, width, val, Physical),
      BorderBottomWidth(val) => property!(border_bottom, width, val, Physical),
      BorderLeftWidth(val) => property!(border_left, width, val, Physical),
      BorderRightWidth(val) => property!(border_right, width, val, Physical),
      BorderBlockStartWidth(val) => property!(border_block_start, width, val, Logical),
      BorderBlockEndWidth(val) => property!(border_block_end, width, val, Logical),
      BorderBlockWidth(val) => {
        property!(border_block_start, width, val.start, Logical);
        property!(border_block_end, width, val.end, Logical);
      }
      BorderInlineStartWidth(val) => property!(border_inline_start, width, val, Logical),
      BorderInlineEndWidth(val) => property!(border_inline_end, width, val, Logical),
      BorderInlineWidth(val) => {
        property!(border_inline_start, width, val.start, Logical);
        property!(border_inline_end, width, val.end, Logical);
      }
      BorderTopStyle(val) => property!(border_top, style, val, Physical),
      BorderBottomStyle(val) => property!(border_bottom, style, val, Physical),
      BorderLeftStyle(val) => property!(border_left, style, val, Physical),
      BorderRightStyle(val) => property!(border_right, style, val, Physical),
      BorderBlockStartStyle(val) => property!(border_block_start, style, val, Logical),
      BorderBlockEndStyle(val) => property!(border_block_end, style, val, Logical),
      BorderBlockStyle(val) => {
        property!(border_block_start, style, val.start, Logical);
        property!(border_block_end, style, val.end, Logical);
      }
      BorderInlineStartStyle(val) => property!(border_inline_start, style, val, Logical),
      BorderInlineEndStyle(val) => property!(border_inline_end, style, val, Logical),
      BorderInlineStyle(val) => {
        property!(border_inline_start, style, val.start, Logical);
        property!(border_inline_end, style, val.end, Logical);
      }
      BorderTop(val) => set_border!(border_top, val, Physical),
      BorderBottom(val) => set_border!(border_bottom, val, Physical),
      BorderLeft(val) => set_border!(border_left, val, Physical),
      BorderRight(val) => set_border!(border_right, val, Physical),
      BorderBlockStart(val) => set_border!(border_block_start, val, Logical),
      BorderBlockEnd(val) => set_border!(border_block_end, val, Logical),
      BorderInlineStart(val) => set_border!(border_inline_start, val, Logical),
      BorderInlineEnd(val) => set_border!(border_inline_end, val, Logical),
      BorderBlock(val) => {
        set_border!(border_block_start, val, Logical);
        set_border!(border_block_end, val, Logical);
      }
      BorderInline(val) => {
        set_border!(border_inline_start, val, Logical);
        set_border!(border_inline_end, val, Logical);
      }
      BorderWidth(val) => {
        self.border_top.width = Some(val.top.clone());
        self.border_right.width = Some(val.right.clone());
        self.border_bottom.width = Some(val.bottom.clone());
        self.border_left.width = Some(val.left.clone());
        self.border_block_start.width = None;
        self.border_block_end.width = None;
        self.border_inline_start.width = None;
        self.border_inline_end.width = None;
        self.has_any = true;
      }
      BorderStyle(val) => {
        self.border_top.style = Some(val.top.clone());
        self.border_right.style = Some(val.right.clone());
        self.border_bottom.style = Some(val.bottom.clone());
        self.border_left.style = Some(val.left.clone());
        self.border_block_start.style = None;
        self.border_block_end.style = None;
        self.border_inline_start.style = None;
        self.border_inline_end.style = None;
        self.has_any = true;
      }
      BorderColor(val) => {
        self.border_top.color = Some(val.top.clone());
        self.border_right.color = Some(val.right.clone());
        self.border_bottom.color = Some(val.bottom.clone());
        self.border_left.color = Some(val.left.clone());
        self.border_block_start.color = None;
        self.border_block_end.color = None;
        self.border_inline_start.color = None;
        self.border_inline_end.color = None;
        self.has_any = true;
      }
      Border(val) => {
        // dest.clear();
        self.border_top.set_border(val);
        self.border_bottom.set_border(val);
        self.border_left.set_border(val);
        self.border_right.set_border(val);
        self.border_block_start.reset();
        self.border_block_end.reset();
        self.border_inline_start.reset();
        self.border_inline_end.reset();

        // Setting the `border` property resets `border-image`.
        self.border_image_handler.reset();
        self.has_any = true;
      }
      Unparsed(val) if is_border_property(&val.property_id) => {
        self.flush(dest, context);
        self.flush_unparsed(&val, dest, context);
      }
      _ => {
        if self.border_image_handler.will_flush(property) {
          self.flush(dest, context);
        }

        return self.border_image_handler.handle_property(property, dest, context)
          || self.border_radius_handler.handle_property(property, dest, context);
      }
    }

    true
  }

  fn finalize(&mut self, dest: &mut DeclarationList<'i>, context: &mut PropertyHandlerContext<'i, '_>) {
    self.flush(dest, context);
    self.border_image_handler.finalize(dest, context);
    self.border_radius_handler.finalize(dest, context);
  }
}

impl<'i> BorderHandler<'i> {
  fn flush(&mut self, dest: &mut DeclarationList, context: &mut PropertyHandlerContext<'i, '_>) {
    if !self.has_any {
      return;
    }

    self.has_any = false;

    let logical_supported = context.is_supported(Feature::LogicalBorders);
    let logical_shorthand_supported = context.is_supported(Feature::LogicalBorderShorthand);
    macro_rules! logical_prop {
      ($ltr: ident, $ltr_key: ident, $rtl: ident, $rtl_key: ident, $val: expr) => {{
        context.add_logical_rule(Property::$ltr($val.clone()), Property::$rtl($val.clone()));
      }};
    }

    macro_rules! fallbacks {
      ($prop: ident => $val: expr) => {{
        let mut val = $val;
        if let Some(targets) = self.targets {
          let fallbacks = val.get_fallbacks(targets);
          for fallback in fallbacks {
            dest.push(Property::$prop(fallback))
          }
        }
        dest.push(Property::$prop(val))
      }};
    }

    macro_rules! prop {
      (BorderInlineStart => $val: expr) => {
        if logical_supported {
          fallbacks!(BorderInlineStart => $val);
        } else {
          logical_prop!(BorderLeft, border_left, BorderRight, border_right, $val);
        }
      };
      (BorderInlineStartWidth => $val: expr) => {
        if logical_supported {
          dest.push(Property::BorderInlineStartWidth($val));
        } else {
          logical_prop!(BorderLeftWidth, border_left_width, BorderRightWidth, border_right_width, $val);
        }
      };
      (BorderInlineStartColor => $val: expr) => {
        if logical_supported {
          fallbacks!(BorderInlineStartColor => $val);
        } else {
          logical_prop!(BorderLeftColor, border_left_color, BorderRightColor, border_right_color, $val);
        }
      };
      (BorderInlineStartStyle => $val: expr) => {
        if logical_supported {
          dest.push(Property::BorderInlineStartStyle($val));
        } else {
          logical_prop!(BorderLeftStyle, border_left_style, BorderRightStyle, border_right_style, $val);
        }
      };
      (BorderInlineEnd => $val: expr) => {
        if logical_supported {
          fallbacks!(BorderInlineEnd => $val);
        } else {
          logical_prop!(BorderRight, border_right, BorderLeft, border_left, $val);
        }
      };
      (BorderInlineEndWidth => $val: expr) => {
        if logical_supported {
          dest.push(Property::BorderInlineEndWidth($val));
        } else {
          logical_prop!(BorderRightWidth, border_right_width, BorderLeftWidth, border_left_width, $val);
        }
      };
      (BorderInlineEndColor => $val: expr) => {
        if logical_supported {
          fallbacks!(BorderInlineEndColor => $val);
        } else {
          logical_prop!(BorderRightColor, border_right_color, BorderLeftColor, border_left_color, $val);
        }
      };
      (BorderInlineEndStyle => $val: expr) => {
        if logical_supported {
          dest.push(Property::BorderInlineEndStyle($val));
        } else {
          logical_prop!(BorderRightStyle, border_right_style, BorderLeftStyle, border_left_style, $val);
        }
      };
      (BorderBlockStart => $val: expr) => {
        if logical_supported {
          fallbacks!(BorderBlockStart => $val);
        } else {
          fallbacks!(BorderTop => $val);
        }
      };
      (BorderBlockStartWidth => $val: expr) => {
        if logical_supported {
          dest.push(Property::BorderBlockStartWidth($val));
        } else {
          dest.push(Property::BorderTopWidth($val));
        }
      };
      (BorderBlockStartColor => $val: expr) => {
        if logical_supported {
          fallbacks!(BorderBlockStartColor => $val);
        } else {
          fallbacks!(BorderTopColor => $val);
        }
      };
      (BorderBlockStartStyle => $val: expr) => {
        if logical_supported {
          dest.push(Property::BorderBlockStartStyle($val));
        } else {
          dest.push(Property::BorderTopStyle($val));
        }
      };
      (BorderBlockEnd => $val: expr) => {
        if logical_supported {
          fallbacks!(BorderBlockEnd => $val);
        } else {
          fallbacks!(BorderBottom => $val);
        }
      };
      (BorderBlockEndWidth => $val: expr) => {
        if logical_supported {
          dest.push(Property::BorderBlockEndWidth($val));
        } else {
          dest.push(Property::BorderBottomWidth($val));
        }
      };
      (BorderBlockEndColor => $val: expr) => {
        if logical_supported {
          fallbacks!(BorderBlockEndColor => $val);
        } else {
          fallbacks!(BorderBottomColor => $val);
        }
      };
      (BorderBlockEndStyle => $val: expr) => {
        if logical_supported {
          dest.push(Property::BorderBlockEndStyle($val));
        } else {
          dest.push(Property::BorderBottomStyle($val));
        }
      };
      (BorderLeftColor => $val: expr) => {
        fallbacks!(BorderLeftColor => $val);
      };
      (BorderRightColor => $val: expr) => {
        fallbacks!(BorderRightColor => $val);
      };
      (BorderTopColor => $val: expr) => {
        fallbacks!(BorderTopColor => $val);
      };
      (BorderBottomColor => $val: expr) => {
        fallbacks!(BorderBottomColor => $val);
      };
      (BorderColor => $val: expr) => {
        fallbacks!(BorderColor => $val);
      };
      (BorderBlockColor => $val: expr) => {
        fallbacks!(BorderBlockColor => $val);
      };
      (BorderInlineColor => $val: expr) => {
        fallbacks!(BorderInlineColor => $val);
      };
      (BorderLeft => $val: expr) => {
        fallbacks!(BorderLeft => $val);
      };
      (BorderRight => $val: expr) => {
        fallbacks!(BorderRight => $val);
      };
      (BorderTop => $val: expr) => {
        fallbacks!(BorderTop => $val);
      };
      (BorderBottom => $val: expr) => {
        fallbacks!(BorderBottom => $val);
      };
      (BorderBlockStart => $val: expr) => {
        fallbacks!(BorderBlockStart => $val);
      };
      (BorderBlockEnd => $val: expr) => {
        fallbacks!(BorderBlockEnd => $val);
      };
      (BorderInlineStart => $val: expr) => {
        fallbacks!(BorderInlineStart => $val);
      };
      (BorderInlineEnd => $val: expr) => {
        fallbacks!(BorderInlineEnd => $val);
      };
      (BorderInline => $val: expr) => {
        fallbacks!(BorderInline => $val);
      };
      (BorderBlock => $val: expr) => {
        fallbacks!(BorderBlock => $val);
      };
      (Border => $val: expr) => {
        fallbacks!(Border => $val);
      };
      ($prop: ident => $val: expr) => {
        dest.push(Property::$prop($val))
      };
    }

    macro_rules! flush_category {
      (
        $block_start_prop: ident,
        $block_start_width: ident,
        $block_start_style: ident,
        $block_start_color: ident,
        $block_start: expr,
        $block_end_prop: ident,
        $block_end_width: ident,
        $block_end_style: ident,
        $block_end_color: ident,
        $block_end: expr,
        $inline_start_prop: ident,
        $inline_start_width: ident,
        $inline_start_style: ident,
        $inline_start_color: ident,
        $inline_start: expr,
        $inline_end_prop: ident,
        $inline_end_width: ident,
        $inline_end_style: ident,
        $inline_end_color: ident,
        $inline_end: expr,
        $is_logical: expr
      ) => {
        macro_rules! shorthand {
          ($prop: ident, $key: ident) => {{
            let has_prop = $block_start.$key.is_some() && $block_end.$key.is_some() && $inline_start.$key.is_some() && $inline_end.$key.is_some();
            if has_prop {
              if !$is_logical || ($block_start.$key == $block_end.$key && $block_end.$key == $inline_start.$key && $inline_start.$key == $inline_end.$key) {
                let rect = $prop {
                  top: std::mem::take(&mut $block_start.$key).unwrap(),
                  right: std::mem::take(&mut $inline_end.$key).unwrap(),
                  bottom: std::mem::take(&mut $block_end.$key).unwrap(),
                  left: std::mem::take(&mut $inline_start.$key).unwrap()
                };
                prop!($prop => rect);
              }
            }
          }};
        }

        macro_rules! logical_shorthand {
          ($prop: ident, $key: ident, $start: expr, $end: expr) => {{
            let has_prop = $start.$key.is_some() && $end.$key.is_some();
            if has_prop {
              prop!($prop => $prop {
                start: std::mem::take(&mut $start.$key).unwrap(),
                end: std::mem::take(&mut $end.$key).unwrap(),
              });
              $end.$key = None;
            }
            has_prop
          }};
        }

        if $block_start.is_valid() && $block_end.is_valid() && $inline_start.is_valid() && $inline_end.is_valid() {
          let top_eq_bottom = $block_start == $block_end;
          let left_eq_right = $inline_start == $inline_end;
          let top_eq_left = $block_start == $inline_start;
          let top_eq_right = $block_start == $inline_end;
          let bottom_eq_left = $block_end == $inline_start;
          let bottom_eq_right = $block_end == $inline_end;

          macro_rules! is_eq {
            ($key: ident) => {
              $block_start.$key == $block_end.$key &&
              $inline_start.$key == $inline_end.$key &&
              $inline_start.$key == $block_start.$key
            };
          }

          macro_rules! prop_diff {
            ($border: expr, $fallback: expr, $border_fallback: literal) => {
              if !$is_logical && is_eq!(color) && is_eq!(style) {
                prop!(Border => $border.to_border());
                shorthand!(BorderWidth, width);
              } else if !$is_logical && is_eq!(width) && is_eq!(style) {
                prop!(Border => $border.to_border());
                shorthand!(BorderColor, color);
              } else if !$is_logical && is_eq!(width) && is_eq!(color) {
                prop!(Border => $border.to_border());
                shorthand!(BorderStyle, style);
              } else {
                if $border_fallback {
                  prop!(Border => $border.to_border());
                }
                $fallback
              }
            };
          }

          macro_rules! side_diff {
            ($border: expr, $other: expr, $prop: ident, $width: ident, $style: ident, $color: ident) => {
              let eq_width = $border.width == $other.width;
              let eq_style = $border.style == $other.style;
              let eq_color = $border.color == $other.color;

              // If only one of the sub-properties is different, only emit that.
              // Otherwise, emit the full border value.
              if eq_width && eq_style {
                prop!($color => $other.color.clone().unwrap());
              } else if eq_width && eq_color {
                prop!($style => $other.style.clone().unwrap());
              } else if eq_style && eq_color {
                prop!($width => $other.width.clone().unwrap());
              } else {
                prop!($prop => $other.to_border());
              }
            };
          }

          if top_eq_bottom && top_eq_left && top_eq_right {
            prop!(Border => $block_start.to_border());
          } else if top_eq_bottom && top_eq_left {
            prop!(Border => $block_start.to_border());
            side_diff!($block_start, $inline_end, $inline_end_prop, $inline_end_width, $inline_end_style, $inline_end_color);
          } else if top_eq_bottom && top_eq_right {
            prop!(Border => $block_start.to_border());
            side_diff!($block_start, $inline_start, $inline_start_prop, $inline_start_width, $inline_start_style, $inline_start_color);
          } else if left_eq_right && bottom_eq_left {
            prop!(Border => $inline_start.to_border());
            side_diff!($inline_start, $block_start, $block_start_prop, $block_start_width, $block_start_style, $block_start_color);
          } else if left_eq_right && top_eq_left {
            prop!(Border => $inline_start.to_border());
            side_diff!($inline_start, $block_end, $block_end_prop, $block_end_width, $block_end_style, $block_end_color);
          } else if top_eq_bottom {
            prop_diff!($block_start, {
              // Try to use border-inline shorthands for the opposide direction if possible.
              let mut handled = false;
              if $is_logical {
                let mut diff = 0;
                if $inline_start.width != $block_start.width || $inline_end.width != $block_start.width {
                  diff += 1;
                }
                if $inline_start.style != $block_start.style || $inline_end.style != $block_start.style {
                  diff += 1;
                }
                if $inline_start.color != $block_start.color || $inline_end.color != $block_start.color {
                  diff += 1;
                }

                if diff == 1 {
                  if $inline_start.width != $block_start.width {
                    prop!(BorderInlineWidth => BorderInlineWidth {
                      start: $inline_start.width.clone().unwrap(),
                      end: $inline_end.width.clone().unwrap(),
                    });
                    handled = true;
                  } else if $inline_start.style != $block_start.style {
                    prop!(BorderInlineStyle => BorderInlineStyle {
                      start: $inline_start.style.clone().unwrap(),
                      end: $inline_end.style.clone().unwrap()
                    });
                    handled = true;
                  } else if $inline_start.color != $block_start.color {
                    prop!(BorderInlineColor => BorderInlineColor {
                      start: $inline_start.color.clone().unwrap(),
                      end: $inline_end.color.clone().unwrap()
                    });
                    handled = true;
                  }
                } else if diff > 1 && $inline_start.width == $inline_end.width && $inline_start.style == $inline_end.style && $inline_start.color == $inline_end.color {
                  prop!(BorderInline => $inline_start.to_border());
                  handled = true;
                }
              }

              if !handled {
                side_diff!($block_start, $inline_start, $inline_start_prop, $inline_start_width, $inline_start_style, $inline_start_color);
                side_diff!($block_start, $inline_end, $inline_end_prop, $inline_end_width, $inline_end_style, $inline_end_color);
              }
            }, true);
          } else if left_eq_right {
            prop_diff!($inline_start, {
              // We know already that top != bottom, so no need to try to use border-block.
              side_diff!($inline_start, $block_start, $block_start_prop, $block_start_width, $block_start_style, $block_start_color);
              side_diff!($inline_start, $block_end, $block_end_prop, $block_end_width, $block_end_style, $block_end_color);
            }, true);
          } else if bottom_eq_right {
            prop_diff!($block_end, {
              side_diff!($block_end, $block_start, $block_start_prop, $block_start_width, $block_start_style, $block_start_color);
              side_diff!($block_end, $inline_start, $inline_start_prop, $inline_start_width, $inline_start_style, $inline_start_color);
            }, true);
          } else {
            prop_diff!($block_start, {
              prop!($block_start_prop => $block_start.to_border());
              prop!($block_end_prop => $block_end.to_border());
              prop!($inline_start_prop => $inline_start.to_border());
              prop!($inline_end_prop => $inline_end.to_border());
            }, false);
          }
        } else {
          shorthand!(BorderStyle, style);
          shorthand!(BorderWidth, width);
          shorthand!(BorderColor, color);

          macro_rules! side {
            ($val: expr, $shorthand: ident, $width: ident, $style: ident, $color: ident) => {
              if $val.is_valid() {
                prop!($shorthand => $val.to_border());
              } else {
                if let Some(style) = &$val.style {
                  prop!($style => style.clone());
                }

                if let Some(width) = &$val.width {
                  prop!($width => width.clone());
                }

                if let Some(color) = &$val.color {
                  prop!($color => color.clone());
                }
              }
            };
          }

          if $is_logical && $block_start == $block_end && $block_start.is_valid() {
            if logical_supported {
              if logical_shorthand_supported {
                prop!(BorderBlock => $block_start.to_border());
              } else {
                prop!(BorderBlockStart => $block_start.to_border());
                prop!(BorderBlockEnd => $block_start.to_border());
              }
            } else {
              prop!(BorderTop => $block_start.to_border());
              prop!(BorderBottom => $block_start.to_border());
            }
          } else {
            if $is_logical && logical_shorthand_supported && !$block_start.is_valid() && !$block_end.is_valid() {
              logical_shorthand!(BorderBlockStyle, style, $block_start, $block_end);
              logical_shorthand!(BorderBlockWidth, width, $block_start, $block_end);
              logical_shorthand!(BorderBlockColor, color, $block_start, $block_end);
            }

            side!($block_start, $block_start_prop, $block_start_width, $block_start_style, $block_start_color);
            side!($block_end, $block_end_prop, $block_end_width, $block_end_style, $block_end_color);
          }

          if $is_logical && $inline_start == $inline_end && $inline_start.is_valid() {
            if logical_supported {
              if logical_shorthand_supported {
                prop!(BorderInline => $inline_start.to_border());
              } else {
                prop!(BorderInlineStart => $inline_start.to_border());
                prop!(BorderInlineEnd => $inline_start.to_border());
              }
            } else {
              prop!(BorderLeft => $inline_start.to_border());
              prop!(BorderRight => $inline_start.to_border());
            }
          } else {
            if $is_logical && !$inline_start.is_valid() && !$inline_end.is_valid() {
              if logical_shorthand_supported {
                logical_shorthand!(BorderInlineStyle, style, $inline_start, $inline_end);
                logical_shorthand!(BorderInlineWidth, width, $inline_start, $inline_end);
                logical_shorthand!(BorderInlineColor, color, $inline_start, $inline_end);
              } else {
                // If both values of an inline logical property are equal, then we can just convert them to physical properties.
                macro_rules! inline_prop {
                  ($key: ident, $left: ident, $right: ident) => {
                    if $inline_start.$key.is_some() && $inline_start.$key == $inline_end.$key {
                      prop!($left => std::mem::take(&mut $inline_start.$key).unwrap());
                      prop!($right => std::mem::take(&mut $inline_end.$key).unwrap());
                    }
                  }
                }

                inline_prop!(style, BorderLeftStyle, BorderRightStyle);
                inline_prop!(width, BorderLeftWidth, BorderRightWidth);
                inline_prop!(color, BorderLeftColor, BorderRightColor);
              }
            }

            side!($inline_start, $inline_start_prop, $inline_start_width, $inline_start_style, $inline_start_color);
            side!($inline_end, $inline_end_prop, $inline_end_width, $inline_end_style, $inline_end_color);
          }
        }
      };
    }

    flush_category!(
      BorderTop,
      BorderTopWidth,
      BorderTopStyle,
      BorderTopColor,
      self.border_top,
      BorderBottom,
      BorderBottomWidth,
      BorderBottomStyle,
      BorderBottomColor,
      self.border_bottom,
      BorderLeft,
      BorderLeftWidth,
      BorderLeftStyle,
      BorderLeftColor,
      self.border_left,
      BorderRight,
      BorderRightWidth,
      BorderRightStyle,
      BorderRightColor,
      self.border_right,
      false
    );

    flush_category!(
      BorderBlockStart,
      BorderBlockStartWidth,
      BorderBlockStartStyle,
      BorderBlockStartColor,
      self.border_block_start,
      BorderBlockEnd,
      BorderBlockEndWidth,
      BorderBlockEndStyle,
      BorderBlockEndColor,
      self.border_block_end,
      BorderInlineStart,
      BorderInlineStartWidth,
      BorderInlineStartStyle,
      BorderInlineStartColor,
      self.border_inline_start,
      BorderInlineEnd,
      BorderInlineEndWidth,
      BorderInlineEndStyle,
      BorderInlineEndColor,
      self.border_inline_end,
      true
    );

    self.border_top.reset();
    self.border_bottom.reset();
    self.border_left.reset();
    self.border_right.reset();
    self.border_block_start.reset();
    self.border_block_end.reset();
    self.border_inline_start.reset();
    self.border_inline_end.reset();
  }

  fn flush_unparsed(
    &mut self,
    unparsed: &UnparsedProperty<'i>,
    dest: &mut DeclarationList<'i>,
    context: &mut PropertyHandlerContext<'i, '_>,
  ) {
    let logical_supported = context.is_supported(Feature::LogicalBorders);
    if logical_supported {
      let mut unparsed = unparsed.clone();
      context.add_unparsed_fallbacks(&mut unparsed);
      dest.push(Property::Unparsed(unparsed));
      return;
    }

    macro_rules! prop {
      ($id: ident) => {{
        let mut unparsed = unparsed.with_property_id(PropertyId::$id);
        context.add_unparsed_fallbacks(&mut unparsed);
        dest.push(Property::Unparsed(unparsed));
      }};
    }

    macro_rules! logical_prop {
      ($ltr: ident, $ltr_key: ident, $rtl: ident, $rtl_key: ident) => {{
        context.add_logical_rule(
          Property::Unparsed(unparsed.with_property_id(PropertyId::$ltr)),
          Property::Unparsed(unparsed.with_property_id(PropertyId::$rtl)),
        );
      }};
    }

    use PropertyId::*;
    match &unparsed.property_id {
      BorderInlineStart => logical_prop!(BorderLeft, border_left, BorderRight, border_right),
      BorderInlineStartWidth => {
        logical_prop!(BorderLeftWidth, border_left_width, BorderRightWidth, border_right_width)
      }
      BorderInlineStartColor => {
        logical_prop!(BorderLeftColor, border_left_color, BorderRightColor, border_right_color)
      }
      BorderInlineStartStyle => {
        logical_prop!(BorderLeftStyle, border_left_style, BorderRightStyle, border_right_style)
      }
      BorderInlineEnd => logical_prop!(BorderRight, border_right, BorderLeft, border_left),
      BorderInlineEndWidth => {
        logical_prop!(BorderRightWidth, border_right_width, BorderLeftWidth, border_left_width)
      }
      BorderInlineEndColor => {
        logical_prop!(BorderRightColor, border_right_color, BorderLeftColor, border_left_color)
      }
      BorderInlineEndStyle => {
        logical_prop!(BorderRightStyle, border_right_style, BorderLeftStyle, border_left_style)
      }
      BorderBlockStart => prop!(BorderTop),
      BorderBlockStartWidth => prop!(BorderTopWidth),
      BorderBlockStartColor => prop!(BorderTopColor),
      BorderBlockStartStyle => prop!(BorderTopStyle),
      BorderBlockEnd => prop!(BorderBottom),
      BorderBlockEndWidth => prop!(BorderBottomWidth),
      BorderBlockEndColor => prop!(BorderBottomColor),
      BorderBlockEndStyle => prop!(BorderBottomStyle),
      _ => {
        let mut unparsed = unparsed.clone();
        context.add_unparsed_fallbacks(&mut unparsed);
        dest.push(Property::Unparsed(unparsed));
      }
    }
  }
}

fn is_border_property(property_id: &PropertyId) -> bool {
  match property_id {
    PropertyId::BorderTopColor
    | PropertyId::BorderBottomColor
    | PropertyId::BorderLeftColor
    | PropertyId::BorderRightColor
    | PropertyId::BorderBlockStartColor
    | PropertyId::BorderBlockEndColor
    | PropertyId::BorderBlockColor
    | PropertyId::BorderInlineStartColor
    | PropertyId::BorderInlineEndColor
    | PropertyId::BorderInlineColor
    | PropertyId::BorderTopWidth
    | PropertyId::BorderBottomWidth
    | PropertyId::BorderLeftWidth
    | PropertyId::BorderRightWidth
    | PropertyId::BorderBlockStartWidth
    | PropertyId::BorderBlockEndWidth
    | PropertyId::BorderBlockWidth
    | PropertyId::BorderInlineStartWidth
    | PropertyId::BorderInlineEndWidth
    | PropertyId::BorderInlineWidth
    | PropertyId::BorderTopStyle
    | PropertyId::BorderBottomStyle
    | PropertyId::BorderLeftStyle
    | PropertyId::BorderRightStyle
    | PropertyId::BorderBlockStartStyle
    | PropertyId::BorderBlockEndStyle
    | PropertyId::BorderBlockStyle
    | PropertyId::BorderInlineStartStyle
    | PropertyId::BorderInlineEndStyle
    | PropertyId::BorderInlineStyle
    | PropertyId::BorderTop
    | PropertyId::BorderBottom
    | PropertyId::BorderLeft
    | PropertyId::BorderRight
    | PropertyId::BorderBlockStart
    | PropertyId::BorderBlockEnd
    | PropertyId::BorderInlineStart
    | PropertyId::BorderInlineEnd
    | PropertyId::BorderBlock
    | PropertyId::BorderInline
    | PropertyId::BorderWidth
    | PropertyId::BorderStyle
    | PropertyId::BorderColor
    | PropertyId::Border => true,
    _ => false,
  }
}
