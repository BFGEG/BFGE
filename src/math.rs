//! Engine math: `glam` re-exports and view-projection helpers.
//!
//! The module wraps `glam` in pure builder functions used to assemble the
//! view and projection matrices of the render frame and to compose them with
//! the scene's model transform. It depends on no other engine module: it
//! accepts only primitive math types.

pub use glam::{Mat4, Quat, Vec3};

/// Near clipping plane distance (world units) used by [`perspective`].
const NEAR: f32 = 0.1;
/// Far clipping plane distance (world units) used by [`perspective`].
const FAR: f32 = 100.0;
/// World up axis used to build the view matrix in [`view`].
const UP: Vec3 = Vec3::Y;

/// Builds the view matrix (world -> view) for a camera at `position`
/// looking along `direction`, with `+Y` as the world up axis.
pub fn view(position: Vec3, direction: Vec3) -> Mat4 {
    Mat4::look_at_rh(position, position + direction, UP)
}

/// Builds a right-handed perspective projection matrix (view -> clip) with a
/// `0..1` depth range, matching the WebGPU/wgpu convention.
///
/// `fov_y` is the vertical field of view in radians and `aspect_ratio` is
/// `width / height`.
pub fn perspective(fov_y: f32, aspect_ratio: f32) -> Mat4 {
    Mat4::perspective_rh(fov_y, aspect_ratio, NEAR, FAR)
}

/// Composes a model-view-projection matrix as `projection * view * model`.
pub fn mvp(model: Mat4, view: Mat4, projection: Mat4) -> Mat4 {
    projection * view * model
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn view_moves_the_camera_position_to_the_origin() {
        let view_matrix = view(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));

        let origin = view_matrix.transform_point3(Vec3::new(0.0, 0.0, 5.0));

        assert!(origin.abs_diff_eq(Vec3::ZERO, EPSILON));
    }

    #[test]
    fn perspective_maps_the_near_plane_to_zero_depth() {
        let projection = perspective(60.0_f32.to_radians(), 1.0);

        let near = projection.project_point3(Vec3::new(0.0, 0.0, -NEAR));

        assert!(near.z.abs() < EPSILON);
    }

    #[test]
    fn perspective_maps_the_far_plane_to_unit_depth() {
        let projection = perspective(60.0_f32.to_radians(), 1.0);

        let far = projection.project_point3(Vec3::new(0.0, 0.0, -FAR));

        assert!((far.z - 1.0).abs() < EPSILON);
    }

    #[test]
    fn mvp_of_identities_is_identity() {
        let composed = mvp(Mat4::IDENTITY, Mat4::IDENTITY, Mat4::IDENTITY);

        assert!(composed.abs_diff_eq(Mat4::IDENTITY, EPSILON));
    }
}
