use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct VertexPayload {
    pub position: Vec3,
    pub color: Option<Vec4>,
    pub weight: Option<f32>,
    pub tag: Option<u32>,
}

impl Default for VertexPayload {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            color: None,
            weight: None,
            tag: None,
        }
    }
}

impl VertexPayload {
    pub fn lerp(&self, other: &Self, factor: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, factor),
            color: match (self.color, other.color) {
                (Some(left), Some(right)) => Some(left.lerp(right, factor)),
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            },
            weight: match (self.weight, other.weight) {
                (Some(left), Some(right)) => Some(left + (right - left) * factor),
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            },
            tag: if factor < 0.5 { self.tag } else { other.tag },
        }
    }

    pub(crate) fn average(values: &[Self]) -> Self {
        if values.is_empty() {
            return Self::default();
        }

        let position = values
            .iter()
            .fold(Vec3::ZERO, |acc, value| acc + value.position)
            / values.len() as f32;
        let color_values = values
            .iter()
            .filter_map(|value| value.color)
            .collect::<Vec<_>>();

        let (weight_sum, weight_count) =
            values.iter().fold((0.0, 0usize), |(sum, count), value| {
                if let Some(weight) = value.weight {
                    (sum + weight, count + 1)
                } else {
                    (sum, count)
                }
            });

        let tag = values.first().and_then(|value| value.tag).filter(|tag| {
            values
                .iter()
                .all(|candidate| candidate.tag == Some(*tag) || candidate.tag.is_none())
        });

        Self {
            position,
            color: (!color_values.is_empty()).then_some(
                color_values
                    .iter()
                    .copied()
                    .fold(Vec4::ZERO, |acc, value| acc + value)
                    / color_values.len() as f32,
            ),
            weight: (weight_count > 0).then_some(weight_sum / weight_count as f32),
            tag,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect, Default)]
pub struct LoopAttributes {
    pub uv: Option<Vec2>,
    pub normal: Option<Vec3>,
    pub tangent: Option<Vec4>,
}

impl LoopAttributes {
    pub fn lerp(&self, other: &Self, factor: f32) -> Self {
        Self {
            uv: match (self.uv, other.uv) {
                (Some(left), Some(right)) => Some(left.lerp(right, factor)),
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            },
            normal: match (self.normal, other.normal) {
                (Some(left), Some(right)) => Some(left.lerp(right, factor).normalize_or_zero()),
                (Some(value), None) | (None, Some(value)) => Some(value.normalize_or_zero()),
                (None, None) => None,
            },
            tangent: match (self.tangent, other.tangent) {
                (Some(left), Some(right)) => {
                    let tangent = left.xyz().lerp(right.xyz(), factor).normalize_or_zero();
                    let sign = if factor < 0.5 { left.w } else { right.w };
                    Some(tangent.extend(sign))
                }
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            },
        }
    }

    pub(crate) fn average(values: &[Self]) -> Self {
        if values.is_empty() {
            return Self::default();
        }

        let uv_values = values
            .iter()
            .filter_map(|value| value.uv)
            .collect::<Vec<_>>();
        let normal_values = values
            .iter()
            .filter_map(|value| value.normal)
            .collect::<Vec<_>>();
        let tangent_values = values
            .iter()
            .filter_map(|value| value.tangent)
            .collect::<Vec<_>>();

        Self {
            uv: (!uv_values.is_empty()).then_some(
                uv_values
                    .iter()
                    .copied()
                    .fold(Vec2::ZERO, |acc, value| acc + value)
                    / uv_values.len() as f32,
            ),
            normal: (!normal_values.is_empty()).then_some(
                normal_values
                    .iter()
                    .copied()
                    .fold(Vec3::ZERO, |acc, value| acc + value)
                    .normalize_or_zero(),
            ),
            tangent: (!tangent_values.is_empty()).then_some({
                let xyz = tangent_values
                    .iter()
                    .fold(Vec3::ZERO, |acc, value| acc + value.xyz())
                    .normalize_or_zero();
                let sign = tangent_values.iter().map(|value| value.w).sum::<f32>()
                    / tangent_values.len() as f32;
                xyz.extend(sign.signum().max(1.0))
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub struct FacePayload {
    pub material: Option<u32>,
    pub region: Option<u32>,
}
