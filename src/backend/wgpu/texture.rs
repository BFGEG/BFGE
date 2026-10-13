//! Depth texture for the wgpu backend.

use crate::render::system::RenderError;

/// Depth format used by the mesh pipeline.
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const DEPTH_TEXTURE_LABEL: &str = "BFGE depth texture";

/// Depth buffer matching the surface size.
///
/// The buffer is recreated whenever the surface is resized. A zero-sized target
/// (minimized window) holds no texture and is skipped during drawing.
#[derive(Debug, Default)]
pub(crate) struct DepthTarget {
    view: Option<wgpu::TextureView>,
    size: Option<(u32, u32)>,
}

impl DepthTarget {
    /// Returns the depth view for `width`x`height`, recreating it on size change.
    ///
    /// Returns `None` for a zero-sized target.
    pub(crate) fn view(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<Option<&wgpu::TextureView>, RenderError> {
        if width == 0 || height == 0 {
            self.view = None;
            self.size = None;
            return Ok(None);
        }
        if self.size != Some((width, height)) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(DEPTH_TEXTURE_LABEL),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.view = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
            self.size = Some((width, height));
        }
        Ok(self.view.as_ref())
    }
}
