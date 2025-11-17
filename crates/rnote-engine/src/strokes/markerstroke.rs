// Imports
use super::Content;
use super::content::GeneratedContentImages;
use crate::Drawable;
use crate::pens::pensconfig::markerconfig::MarkerShape;
use crate::render;
use p2d::bounding_volume::{Aabb, BoundingVolume};
use piet::RenderContext;
use rnote_compose::ext::AabbExt;
use rnote_compose::penpath::Element;
use rnote_compose::shapes::Shapeable;
use rnote_compose::transform::Transformable;
use rnote_compose::{Color, PenPath};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "markerstroke")]
pub struct MarkerStroke {
    #[serde(rename = "path")]
    pub path: PenPath,
    #[serde(rename = "width")]
    pub width: f64,
    #[serde(rename = "shape")]
    pub shape: MarkerShape,
    #[serde(rename = "color")]
    pub color: Color,
    #[serde(skip)]
    hitboxes: Vec<Aabb>,
}

impl Content for MarkerStroke {
    fn gen_images(
        &self,
        viewport: Aabb,
        image_scale: f64,
    ) -> Result<GeneratedContentImages, anyhow::Error> {
        let bounds = self.bounds();
        let partial = !viewport.contains(&bounds);
        let Some(bounds) = viewport.intersection(&bounds) else {
            return Ok(GeneratedContentImages::Partial {
                images: vec![],
                viewport,
            });
        };

        // For markers, render as a single image to avoid self-overlap
        let image = render::Image::gen_with_piet(
            |piet_cx| {
                self.draw_marker_path(piet_cx);
                Ok(())
            },
            bounds,
            image_scale,
        );

        let images = match image {
            Ok(image) => vec![image],
            Err(e) => {
                error!("Generating images for markerstroke failed, Err: {e:?}");
                vec![]
            }
        };

        if partial {
            Ok(GeneratedContentImages::Partial { images, viewport })
        } else {
            Ok(GeneratedContentImages::Full(images))
        }
    }

    fn draw_highlight(
        &self,
        cx: &mut impl piet::RenderContext,
        total_zoom: f64,
    ) -> anyhow::Result<()> {
        const PATH_HIGHLIGHT_MIN_STROKE_WIDTH: f64 = 5.0;
        const DRAW_BOUNDS_THRESHOLD_AREA: f64 = 10_u32.pow(2) as f64;

        let bounds = self.bounds();

        if bounds.scale(total_zoom).volume() < DRAW_BOUNDS_THRESHOLD_AREA {
            cx.fill(
                bounds.to_kurbo_rect(),
                &super::content::CONTENT_HIGHLIGHT_COLOR,
            );
        } else {
            cx.stroke_styled(
                self.outline_path(),
                &super::content::CONTENT_HIGHLIGHT_COLOR,
                (PATH_HIGHLIGHT_MIN_STROKE_WIDTH / total_zoom).max(self.width + 3.0 / total_zoom),
                &piet::StrokeStyle::new()
                    .line_join(piet::LineJoin::Round)
                    .line_cap(piet::LineCap::Round),
            );
        }
        Ok(())
    }

    fn update_geometry(&mut self) {
        self.hitboxes = self.gen_hitboxes_int();
    }
}

impl Drawable for MarkerStroke {
    fn draw(&self, cx: &mut impl piet::RenderContext, _image_scale: f64) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        self.draw_marker_path(cx);
        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}

impl Shapeable for MarkerStroke {
    fn bounds(&self) -> Aabb {
        self.path.bounds().loosened(self.width * 0.5)
    }

    fn hitboxes(&self) -> Vec<Aabb> {
        self.hitboxes.clone()
    }

    fn outline_path(&self) -> kurbo::BezPath {
        self.path.outline_path()
    }
}

impl Transformable for MarkerStroke {
    fn translate(&mut self, offset: na::Vector2<f64>) {
        self.path.translate(offset);
    }
    fn rotate(&mut self, angle: f64, center: na::Point2<f64>) {
        self.path.rotate(angle, center);
    }
    fn scale(&mut self, scale: na::Vector2<f64>) {
        self.path.scale(scale);
        // Using the geometric mean behaves the best when scaling non-uniformly.
        let scale_scalar = (scale[0] * scale[1]).sqrt();
        self.width *= scale_scalar;
    }
}

impl MarkerStroke {
    pub fn new(start: Element, width: f64, shape: MarkerShape, color: Color) -> Self {
        let path = PenPath::new(start);

        Self::from_penpath(path, width, shape, color)
    }

    pub fn from_penpath(path: PenPath, width: f64, shape: MarkerShape, color: Color) -> Self {
        let mut new_markerstroke = Self {
            path,
            width,
            shape,
            color,
            hitboxes: vec![],
        };
        new_markerstroke.update_geometry();

        new_markerstroke
    }

    pub fn extend_w_segments(
        &mut self,
        segments: impl IntoIterator<Item = rnote_compose::penpath::Segment>,
    ) {
        self.path.extend(segments);
    }

    /// Replace the current path with the given new one. The new path must not be empty.
    pub fn replace_path(&mut self, path: PenPath) {
        self.path = path;
        self.update_geometry();
    }

    fn gen_hitboxes_int(&self) -> Vec<Aabb> {
        self.path
            .hitboxes()
            .into_iter()
            .map(|hb| hb.loosened(self.width * 0.5))
            .collect()
    }

    /// Draw the marker path to the given context
    fn draw_marker_path(&self, cx: &mut impl piet::RenderContext) {
        let bez_path = self.path.to_kurbo_flattened(0.1);

        // Convert color to piet Color
        let piet_color = piet::Color::rgba(self.color.r, self.color.g, self.color.b, self.color.a);

        // Create the stroke style based on shape
        let stroke_style = match self.shape {
            MarkerShape::Circular => piet::StrokeStyle::new()
                .line_join(piet::LineJoin::Round)
                .line_cap(piet::LineCap::Round),
            MarkerShape::Rectangular => piet::StrokeStyle::new()
                .line_join(piet::LineJoin::Bevel)
                .line_cap(piet::LineCap::Butt),
        };

        // Draw the stroke
        cx.stroke_styled(bez_path, &piet_color, self.width, &stroke_style);
    }

    pub fn gen_image_for_last_segments(
        &self,
        n_last_segments: usize,
        image_scale: f64,
    ) -> Result<Option<render::Image>, anyhow::Error> {
        let path_len = self.path.segments.len();

        let start_el = self
            .path
            .segments
            .get(path_len.saturating_sub(n_last_segments).saturating_sub(1))
            .map(|s| s.end())
            .unwrap_or(self.path.start);

        let range_path = PenPath::new_w_segments(
            start_el,
            self.path.segments[path_len.saturating_sub(n_last_segments)..]
                .iter()
                .copied(),
        );

        // Calculate bounds for the range path
        let bounds = range_path.bounds().loosened(self.width * 0.5);

        let image = render::Image::gen_with_piet(
            |piet_cx| {
                let bez_path = range_path.to_kurbo_flattened(0.1);

                let piet_color =
                    piet::Color::rgba(self.color.r, self.color.g, self.color.b, self.color.a);

                let stroke_style = match self.shape {
                    MarkerShape::Circular => piet::StrokeStyle::new()
                        .line_join(piet::LineJoin::Round)
                        .line_cap(piet::LineCap::Round),
                    MarkerShape::Rectangular => piet::StrokeStyle::new()
                        .line_join(piet::LineJoin::Bevel)
                        .line_cap(piet::LineCap::Butt),
                };

                piet_cx.stroke_styled(bez_path, &piet_color, self.width, &stroke_style);
                Ok(())
            },
            bounds,
            image_scale,
        )?;

        Ok(Some(image))
    }
}
