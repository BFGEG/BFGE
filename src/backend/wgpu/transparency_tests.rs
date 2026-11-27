use super::*;
use wgpu::util::DeviceExt;

#[test]
#[ignore = "requires a graphics adapter; renders and reads back an offscreen pixel"]
fn alpha_texture_and_material_blend_without_writing_depth() -> Result<(), Box<dyn std::error::Error>>
{
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let mut pipeline = MeshPipeline::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    pipeline.reserve(&device, 4);
    pipeline.write_camera(
        &queue,
        Mat4::IDENTITY,
        SceneLighting {
            ambient: glam::Vec3::ONE,
            directional: crate::scene::lighting::DirectionalLight {
                direction: glam::Vec3::NEG_Z,
                color: glam::Vec3::ZERO,
            },
        },
    );
    for (index, (z, color)) in [
        (-0.25, [1.0, 0.0, 0.0, 0.5]),
        (0.0, [0.0, 1.0, 0.0, 1.0]),
        (-0.25, [1.0, 0.0, 0.0, 0.5]),
        (0.25, [0.0, 0.0, 1.0, 1.0]),
    ]
    .into_iter()
    .enumerate()
    {
        pipeline.write_mesh(
            &queue,
            index as u32,
            Mat4::from_translation(glam::Vec3::new(0.0, 0.0, z)),
            color,
        );
    }
    let vertices = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| GpuVertex {
        position: [x, y, 0.5],
        normal: [0.0, 0.0, 1.0],
        uv: [0.5, 0.5],
    });
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&[0u32, 1, 2, 0, 2, 3]),
        usage: wgpu::BufferUsages::INDEX,
    });
    let extent = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut depth = crate::backend::wgpu::texture::DepthTarget::default();
    let depth_view = depth.view(&device, 1, 1)?.ok_or("missing depth view")?;
    let bind_groups = [128u8, 255].map(|alpha| {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &[255, 255, 255, alpha],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            extent,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline.material_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_bind_group(0, pipeline.camera_bind_group(), &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
        // The first red layer must not prevent the opaque green layer behind it.
        // The last blue layer must fail the depth test against opaque green.
        for index in 0..4 {
            let transparent = index != 1;
            pass.set_pipeline(pipeline.pipeline(transparent));
            pass.set_bind_group(
                1,
                pipeline.mesh_bind_group(),
                &[index * pipeline.mesh_stride()],
            );
            pass.set_bind_group(2, &bind_groups[usize::from(!transparent)], &[]);
            pass.draw_indexed(0..6, 0, 0..1);
        }
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &output,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    output
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = send.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receive.recv()??;
    let data = output.slice(..).get_mapped_range()?;
    // 0.5 material opacity * 128/255 texture alpha, over opaque green.
    for (actual, expected) in data[..4].iter().zip([64u8, 191, 0, 255]) {
        assert!(
            actual.abs_diff(expected) <= 1,
            "unexpected pixel: {:?}",
            &data[..4]
        );
    }
    drop(data);
    output.unmap();
    Ok(())
}
