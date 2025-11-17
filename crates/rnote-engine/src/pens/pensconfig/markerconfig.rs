// Imports
use crate::store::chrono_comp::StrokeLayer;
use rnote_compose::Color;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    num_derive::FromPrimitive,
    num_derive::ToPrimitive,
)]
#[serde(rename = "marker_shape")]
pub enum MarkerShape {
    #[serde(rename = "circular")]
    Circular = 0,
    #[serde(rename = "rectangular")]
    Rectangular,
}

impl Default for MarkerShape {
    fn default() -> Self {
        Self::Circular
    }
}

impl TryFrom<u32> for MarkerShape {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_u32(value).ok_or_else(|| {
            anyhow::anyhow!("MarkerShape try_from::<u32>() for value {} failed", value)
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename = "marker_config")]
pub struct MarkerConfig {
    #[serde(rename = "strength")]
    pub strength: f64,
    #[serde(rename = "width")]
    pub width: f64,
    #[serde(rename = "shape")]
    pub shape: MarkerShape,
    #[serde(rename = "color")]
    pub color: Color,
}

impl Default for MarkerConfig {
    fn default() -> Self {
        Self {
            strength: 0.5,
            width: 15.0,
            shape: MarkerShape::default(),
            color: Color {
                r: 1.0,
                g: 0.9,
                b: 0.0,
                a: 1.0,
            },
        }
    }
}

impl MarkerConfig {
    pub const STRENGTH_MIN: f64 = 0.0;
    pub const STRENGTH_MAX: f64 = 1.0;
    pub const WIDTH_MIN: f64 = 1.0;
    pub const WIDTH_MAX: f64 = 500.0;

    pub(crate) fn layer(&self) -> StrokeLayer {
        StrokeLayer::Highlighter
    }

    /// Get the effective color with strength applied
    pub fn effective_color(&self) -> Color {
        let mut color = self.color;
        color.a *= self.strength;
        color
    }
}
