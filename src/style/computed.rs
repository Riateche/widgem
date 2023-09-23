use tiny_skia::Color;

use super::PhysicalPixels;

#[derive(Debug, Clone)]
pub struct ComputedBorderStyle {
    pub color: Color,
    pub width: PhysicalPixels,
    pub radius: PhysicalPixels,
}
