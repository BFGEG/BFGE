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

/// Builds the view matrix (world -> view) for a camera at `position` looking
/// along `direction`, with `+Y` as the world up axis and `roll` radians of
/// rotation about the view direction.
///
/// A zero `roll` keeps the camera's up axis in the vertical plane of
/// `direction`. Positive `roll` rotates the camera's up vector toward its right
/// vector, so the scene appears rotated counter-clockwise.
pub fn view(position: Vec3, direction: Vec3, roll: f32) -> Mat4 {
    let forward = direction.normalize_or_zero();
    let right = forward.cross(UP).normalize_or_zero();
    let up = right.cross(forward);
    let (sin, cos) = roll.sin_cos();
    Mat4::look_at_rh(position, position + direction, up * cos + right * sin)
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

    const POSITION: Vec3 = Vec3::new(0.0, 0.0, 5.0);
    const FORWARD: Vec3 = Vec3::new(0.0, 0.0, -1.0);

    #[test]
    fn view_moves_the_camera_position_to_the_origin() {
        let view_matrix = view(POSITION, FORWARD, 0.0);

        let origin = view_matrix.transform_point3(POSITION);

        assert!(origin.abs_diff_eq(Vec3::ZERO, EPSILON));
    }

    #[test]
    fn zero_roll_keeps_the_world_up_axis() {
        let view_matrix = view(POSITION, FORWARD, 0.0);

        let up = view_matrix.inverse().transform_vector3(Vec3::Y);

        assert!(up.abs_diff_eq(Vec3::Y, EPSILON));
    }

    #[test]
    fn quarter_turn_roll_rotates_the_up_axis_toward_the_right_axis() {
        let view_matrix = view(POSITION, FORWARD, std::f32::consts::FRAC_PI_2);

        let up = view_matrix.inverse().transform_vector3(Vec3::Y);
        let right = view_matrix.inverse().transform_vector3(Vec3::X);

        assert!(up.abs_diff_eq(Vec3::X, EPSILON));
        assert!(right.abs_diff_eq(-Vec3::Y, EPSILON));
    }

    #[test]
    fn roll_builds_an_orthonormal_frame_for_a_tilted_direction() {
        let view_matrix = view(POSITION, Vec3::new(1.0, -0.5, -1.0), 0.7);

        let rotation = view_matrix.inverse();
        let axes = [
            rotation.transform_vector3(Vec3::X),
            rotation.transform_vector3(Vec3::Y),
            rotation.transform_vector3(Vec3::Z),
        ];

        assert!(view_matrix.is_finite());
        for axis in axes {
            assert!((axis.length() - 1.0).abs() < EPSILON);
        }
        for (index, axis) in axes.iter().enumerate() {
            for other in &axes[index + 1..] {
                assert!(axis.dot(*other).abs() < EPSILON);
            }
        }
    }

    #[test]
    fn full_turn_roll_restores_the_zero_roll_view() {
        let zero_roll = view(POSITION, FORWARD, 0.0);
        let full_turn = view(POSITION, FORWARD, std::f32::consts::TAU);

        assert!(full_turn.abs_diff_eq(zero_roll, EPSILON));
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
