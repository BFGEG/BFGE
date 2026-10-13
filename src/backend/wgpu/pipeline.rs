//! Graphics pipeline for the wgpu backend.

#![expect(dead_code, reason = "Geometry drawing follows in issue #28")]

use crate::render::frame::{RenderCamera, RenderItem};
use crate::render::system::RenderError;

use super::buffer::GpuVertex;
use super::surface::checked_gpu_operation;

const SHADER_LABEL: &str = "BFGE mesh shader";
const CAMERA_LAYOUT_LABEL: &str = "BFGE camera uniforms";
const MESH_LAYOUT_LABEL: &str = "BFGE mesh uniforms";
const PIPELINE_LAYOUT_LABEL: &str = "BFGE mesh pipeline layout";
const PIPELINE_LABEL: &str = "BFGE mesh pipeline";
const VERTEX_ENTRY: &str = "vs_main";
const FRAGMENT_ENTRY: &str = "fs_main";

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub(crate) fn from_camera(camera: &RenderCamera) -> Self {
        Self {
            view_proj: (camera.projection * camera.view).to_cols_array_2d(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MeshUniform {
    pub world: [[f32; 4]; 4],
    pub color: [f32; 4],
}

impl MeshUniform {
    pub(crate) fn from_item(item: &RenderItem, color: [f32; 4]) -> Self {
        Self {
            world: item.world_transform.to_cols_array_2d(),
            color,
        }
    }
}

#[derive(Debug)]
pub(crate) struct MeshPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub camera_layout: wgpu::BindGroupLayout,
    pub mesh_layout: wgpu::BindGroupLayout,
}

impl MeshPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> Result<Self, RenderError> {
        checked_gpu_operation(device, || {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(SHADER_LABEL),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mesh.wgsl").into()),
            });
            let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(CAMERA_LAYOUT_LABEL),
                entries: &[uniform_binding::<CameraUniform>(wgpu::ShaderStages::VERTEX)],
            });
            let mesh_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(MESH_LAYOUT_LABEL),
                entries: &[uniform_binding::<MeshUniform>(
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                )],
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(PIPELINE_LAYOUT_LABEL),
                bind_group_layouts: &[Some(&camera_layout), Some(&mesh_layout)],
                ..Default::default()
            });
            let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(PIPELINE_LABEL),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(VERTEX_ENTRY),
                    compilation_options: Default::default(),
                    buffers: &[Some(GpuVertex::layout())],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(FRAGMENT_ENTRY),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            Self {
                render_pipeline,
                camera_layout,
                mesh_layout,
            }
        })
        .map_err(|error| RenderError::Initialization(format!("create mesh pipeline: {error}")))
    }
}

fn uniform_binding<T>(visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<T>() as u64),
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::math::Mat4;
    use crate::resources::handle::MeshHandle;

    use super::*;

    #[test]
    fn uniform_layout_matches_shader() {
        assert_eq!(std::mem::size_of::<CameraUniform>(), 64);
        assert_eq!(std::mem::size_of::<MeshUniform>(), 80);
        assert_eq!(std::mem::offset_of!(MeshUniform, color), 64);
    }

    #[test]
    fn camera_matrix_applies_view_before_projection() {
        let camera = RenderCamera {
            view: Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0)),
            projection: Mat4::from_scale(glam::Vec3::splat(2.0)),
        };
        let uniform = CameraUniform::from_camera(&camera);

        assert_eq!(
            Mat4::from_cols_array_2d(&uniform.view_proj).transform_point3(glam::Vec3::ZERO),
            glam::Vec3::new(2.0, 4.0, 6.0)
        );
    }

    #[test]
    fn mesh_uniform_preserves_transform_and_color() {
        let item = RenderItem {
            world_transform: Mat4::from_translation(glam::Vec3::new(4.0, 5.0, 6.0)),
            mesh: MeshHandle::new(0),
        };
        let color = [0.2, 0.4, 0.6, 1.0];
        let uniform = MeshUniform::from_item(&item, color);

        assert_eq!(uniform.world, item.world_transform.to_cols_array_2d());
        assert_eq!(uniform.color, color);
    }

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn shader_and_pipeline_pass_gpu_validation() -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        MeshPipeline::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb)?;
        Ok(())
    }
}
