//! Graphics pipeline for the wgpu backend.

use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use glam::Mat3;

use super::buffer::GpuVertex;
use super::texture::DEPTH_FORMAT;
use crate::math::Mat4;
use crate::scene::lighting::SceneLighting;

const MESH_SHADER: wgpu::ShaderModuleDescriptor<'_> = wgpu::include_wgsl!("shaders/mesh.wgsl");
const PIPELINE_LABEL: &str = "BFGE mesh pipeline";
const CAMERA_LAYOUT_LABEL: &str = "BFGE camera bind group layout";
const MESH_LAYOUT_LABEL: &str = "BFGE mesh bind group layout";
const CAMERA_BUFFER_LABEL: &str = "BFGE camera uniform";
const MESH_BUFFER_LABEL: &str = "BFGE mesh uniforms";
const MESH_BUFFER_INITIAL_CAPACITY: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    ambient: [f32; 4],
    light_direction: [f32; 4],
    light_color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct MeshUniform {
    world: [[f32; 4]; 4],
    normal_matrix: [[f32; 4]; 3],
    color: [f32; 4],
}

// Const-evaluated: `MeshUniform` is a non-empty `#[repr(C)]` struct, so this
// is always `Some` and cannot panic at runtime.
const MESH_UNIFORM_SIZE: NonZeroU64 = NonZeroU64::new(size_of::<MeshUniform>() as u64).unwrap();

#[derive(Debug)]
pub(crate) struct MeshPipeline {
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    mesh_layout: wgpu::BindGroupLayout,
    mesh_buffer: wgpu::Buffer,
    mesh_bind_group: wgpu::BindGroup,
    mesh_stride: u32,
    mesh_capacity: u32,
}

impl MeshPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(MESH_SHADER);
        let camera_layout = camera_bind_group_layout(device);
        let mesh_layout = mesh_bind_group_layout(device);
        let pipeline = create_pipeline(device, &shader, format, &camera_layout, &mesh_layout);

        let camera_buffer = uniform_buffer(device, CAMERA_BUFFER_LABEL, size_of::<CameraUniform>());
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(CAMERA_LAYOUT_LABEL),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let mesh_stride = mesh_stride(
            device.limits().min_uniform_buffer_offset_alignment,
            size_of::<MeshUniform>(),
        );
        let mesh_buffer = mesh_uniform_buffer(device, MESH_BUFFER_INITIAL_CAPACITY, mesh_stride);
        let mesh_bind_group = create_mesh_bind_group(device, &mesh_layout, &mesh_buffer);

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            mesh_layout,
            mesh_buffer,
            mesh_bind_group,
            mesh_stride,
            mesh_capacity: MESH_BUFFER_INITIAL_CAPACITY,
        }
    }

    pub(crate) fn mesh_stride(&self) -> u32 {
        self.mesh_stride
    }

    pub(crate) fn camera_bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bind_group
    }

    pub(crate) fn mesh_bind_group(&self) -> &wgpu::BindGroup {
        &self.mesh_bind_group
    }

    pub(crate) fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub(crate) fn write_camera(
        &self,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        lighting: SceneLighting,
    ) {
        let uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            ambient: lighting.ambient.extend(0.0).to_array(),
            light_direction: lighting
                .directional
                .direction
                .normalize_or_zero()
                .extend(0.0)
                .to_array(),
            light_color: lighting.directional.color.extend(0.0).to_array(),
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub(crate) fn reserve(&mut self, device: &wgpu::Device, count: u32) {
        if count <= self.mesh_capacity {
            return;
        }
        self.mesh_capacity = count.max(self.mesh_capacity * 2);
        self.mesh_buffer = mesh_uniform_buffer(device, self.mesh_capacity, self.mesh_stride);
        self.mesh_bind_group = create_mesh_bind_group(device, &self.mesh_layout, &self.mesh_buffer);
    }

    pub(crate) fn write_mesh(&self, queue: &wgpu::Queue, index: u32, world: Mat4, color: [f32; 4]) {
        let uniform = MeshUniform {
            world: world.to_cols_array_2d(),
            normal_matrix: normal_matrix(world),
            color,
        };
        let offset = u64::from(index) * u64::from(self.mesh_stride);
        queue.write_buffer(&self.mesh_buffer, offset, bytemuck::bytes_of(&uniform));
    }
}

fn normal_matrix(world: Mat4) -> [[f32; 4]; 3] {
    let linear = Mat3::from_mat4(world);
    let determinant = linear.determinant();
    let matrix = if determinant.is_finite() && determinant != 0.0 {
        linear.inverse().transpose()
    } else {
        Mat3::ZERO
    };
    matrix
        .to_cols_array_2d()
        .map(|column| [column[0], column[1], column[2], 0.0])
}

fn camera_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(CAMERA_LAYOUT_LABEL),
        entries: &[uniform_entry(0, false)],
    })
}

fn mesh_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(MESH_LAYOUT_LABEL),
        entries: &[uniform_entry(0, true)],
    })
}

fn create_mesh_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(MESH_LAYOUT_LABEL),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: 0,
                size: Some(MESH_UNIFORM_SIZE),
            }),
        }],
    })
}

fn uniform_entry(binding: u32, dynamic: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: dynamic,
            min_binding_size: None,
        },
        count: None,
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    mesh_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(PIPELINE_LABEL),
        bind_group_layouts: &[Some(camera_layout), Some(mesh_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(PIPELINE_LABEL),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(GpuVertex::LAYOUT)],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn mesh_uniform_buffer(device: &wgpu::Device, capacity: u32, stride: u32) -> wgpu::Buffer {
    uniform_buffer(
        device,
        MESH_BUFFER_LABEL,
        capacity as usize * stride as usize,
    )
}

fn uniform_buffer(device: &wgpu::Device, label: &str, size: usize) -> wgpu::Buffer {
    let alignment = wgpu::COPY_BUFFER_ALIGNMENT as usize;
    let padded = size.div_ceil(alignment) * alignment;
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: padded.max(alignment) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn mesh_stride(alignment: u32, size: usize) -> u32 {
    let alignment = alignment as usize;
    (size.div_ceil(alignment) * alignment) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    const MESH_WGSL: &str = include_str!("shaders/mesh.wgsl");

    #[test]
    fn uniforms_are_padded_to_vec4_boundaries() {
        assert_eq!(size_of::<MeshUniform>(), 128);
        assert_eq!(size_of::<CameraUniform>(), 112);
    }

    #[test]
    fn mesh_stride_rounds_up_to_alignment() {
        assert_eq!(mesh_stride(256, 128), 256);
        assert_eq!(mesh_stride(64, 128), 128);
        assert_eq!(mesh_stride(128, 128), 128);
    }

    #[test]
    fn mesh_uniform_size_matches_the_struct() {
        assert_eq!(MESH_UNIFORM_SIZE.get(), size_of::<MeshUniform>() as u64);
    }

    #[test]
    fn normal_matrix_uses_inverse_scale() {
        let columns = normal_matrix(Mat4::from_scale(Vec3::new(2.0, 1.0, 0.5)));

        assert_eq!(columns[0], [0.5, 0.0, 0.0, 0.0]);
        assert_eq!(columns[1], [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(columns[2], [0.0, 0.0, 2.0, 0.0]);
    }

    #[test]
    fn singular_transform_uses_zero_normal_matrix() {
        let columns = normal_matrix(Mat4::from_scale(Vec3::new(0.0, 1.0, 1.0)));

        assert_eq!(columns, [[0.0; 4]; 3]);
    }

    #[test]
    fn mesh_shader_parses_and_validates() {
        let module =
            wgpu::naga::front::wgsl::parse_str(MESH_WGSL).expect("the mesh shader parses as WGSL");
        let mut validator = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .expect("the mesh shader passes naga validation");
    }

    #[test]
    fn mesh_shader_exposes_the_expected_entry_points() {
        let module =
            wgpu::naga::front::wgsl::parse_str(MESH_WGSL).expect("the mesh shader parses as WGSL");

        let entry_points: Vec<&str> = module
            .entry_points
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(entry_points.contains(&"vs_main"));
        assert!(entry_points.contains(&"fs_main"));
    }
}
