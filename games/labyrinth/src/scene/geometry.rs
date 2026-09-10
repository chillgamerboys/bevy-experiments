//! Camera-aware conversion of laid-out, viewport-local UI anchors.

use bevy::prelude::*;

/// Largest in-bounds vertical source crop placing a painted floor at a requested
/// fraction of the output stage. The output sprite itself never leaves the stage.
pub(super) fn floor_aligned_rect(
    size: UVec2,
    source_floor: f32,
    target_floor: f32,
) -> Option<Rect> {
    if size.min_element() == 0
        || !source_floor.is_finite()
        || !target_floor.is_finite()
        || !(0.0..1.0).contains(&source_floor)
        || source_floor == 0.0
        || !(0.0..1.0).contains(&target_floor)
        || target_floor == 0.0
    {
        return None;
    }
    let size = size.as_vec2();
    let floor = size.y * source_floor;
    let height = (floor / target_floor).min((size.y - floor) / (1.0 - target_floor));
    let top = (floor - target_floor * height).max(0.0);
    Some(Rect::from_corners(
        Vec2::new(0.0, top),
        Vec2::new(size.x, (top + height).min(size.y)),
    ))
}

/// World-space basis of the anchor, with local coordinates running left/up.
#[derive(Clone, Copy, Debug)]
pub(super) struct AnchorFrame {
    pub center: Vec2,
    pub right: Vec2,
    pub up: Vec2,
}

impl AnchorFrame {
    pub fn point(self, horizontal: f32, vertical: f32) -> Vec2 {
        self.center + self.right * horizontal + self.up * vertical
    }

    pub fn transform(self, horizontal: f32, vertical: f32, depth: f32) -> Transform {
        Transform {
            translation: self.point(horizontal, vertical).extend(depth),
            rotation: Quat::from_rotation_z(self.right.y.atan2(self.right.x)),
            scale: Vec3::new(1.0, self.right.perp_dot(self.up).signum(), 1.0),
        }
    }

    pub fn size(self) -> Vec2 {
        Vec2::new(self.right.length(), self.up.length())
    }
}

pub(super) fn anchor_frame(
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<AnchorFrame> {
    frame_from_pixels(
        node.size(),
        node.inverse_scale_factor,
        transform,
        camera,
        camera_transform,
    )
}

pub(super) fn frame_from_pixels(
    size: Vec2,
    inverse_scale: f32,
    transform: &UiGlobalTransform,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<AnchorFrame> {
    if !size.is_finite()
        || size.min_element() <= 0.0
        || !inverse_scale.is_finite()
        || inverse_scale <= 0.0
        || transform.try_inverse().is_none()
        || !camera_transform.affine().is_finite()
        || !camera.is_active
        || !camera.clip_from_view().is_finite()
        || camera.clip_from_view().determinant() == 0.0
    {
        return None;
    }
    // UI layout starts at the viewport origin, while the camera conversion takes
    // coordinates relative to the complete render target. Preserve that offset.
    let origin = camera.logical_viewport_rect()?.min;
    let project = |local| {
        let logical = transform.transform_point2(local) * inverse_scale + origin;
        camera.viewport_to_world_2d(camera_transform, logical).ok()
    };
    let center = project(Vec2::ZERO)?;
    let right = project(Vec2::new(size.x * 0.5, 0.0))? - project(Vec2::new(-size.x * 0.5, 0.0))?;
    let up = project(Vec2::new(0.0, -size.y * 0.5))? - project(Vec2::new(0.0, size.y * 0.5))?;
    if !center.is_finite()
        || !right.is_finite()
        || !up.is_finite()
        || right.length_squared() <= f32::EPSILON
        || up.length_squared() <= f32::EPSILON
    {
        return None;
    }
    Some(AnchorFrame { center, right, up })
}
