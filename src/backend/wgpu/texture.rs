//! Render target textures for the wgpu backend.

use crate::platform::WindowSize;
use crate::render::system::RenderError;

use super::surface::checked_gpu_operation;

const DEPTH_TEXTURE_LABEL: &str = "BFGE depth texture";
const DEPTH_VIEW_LABEL: &str = "BFGE depth view";
const ZERO_DEPTH_SIZE: &str = "depth texture requires nonzero dimensions";

pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

#[derive(Debug)]
pub(super) struct DepthTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl DepthTexture {
    pub(super) fn new(device: &wgpu::Device, size: WindowSize) -> Result<Self, RenderError> {
        if size.width == 0 || size.height == 0 {
            Err(RenderError::Render(ZERO_DEPTH_SIZE.to_owned()))
        } else {
            checked_gpu_operation(device, || {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(DEPTH_TEXTURE_LABEL),
                    size: wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(DEPTH_VIEW_LABEL),
                    ..Default::default()
                });
                Self {
                    _texture: texture,
                    view,
                }
            })
            .map_err(|error| RenderError::Render(format!("create depth texture: {error}")))
        }
    }

    pub(super) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn zero_sized_depth_texture_returns_error() -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        assert!(matches!(
            DepthTexture::new(&device, WindowSize::new(0, 240)),
            Err(RenderError::Render(message)) if message == ZERO_DEPTH_SIZE
        ));
        Ok(())
    }

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn depth_textures_at_multiple_sizes_can_be_cleared() -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        for size in [WindowSize::new(320, 240), WindowSize::new(640, 480)] {
            let depth = DepthTexture::new(&device, size)?;
            assert_eq!(depth._texture.width(), size.width);
            assert_eq!(depth._texture.height(), size.height);

            checked_gpu_operation(&device, || {
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                {
                    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: depth.view(),
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        ..Default::default()
                    });
                }
                queue.submit([encoder.finish()]);
            })?;
        }
        Ok(())
    }
}
