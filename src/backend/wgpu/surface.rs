//! Window surface, GPU device, swapchain configuration and presentation.

use std::sync::Arc;

use winit::window::Window as NativeWindow;

use crate::platform::{Window, WindowSize};
use crate::render::system::RenderError;

const DEVICE_LABEL: &str = "BFGE device";
const ENCODER_LABEL: &str = "BFGE frame encoder";
const CLEAR_PASS_LABEL: &str = "BFGE surface clear";
const NO_CONFIGURATION: &str = "adapter does not support the window surface";
const ACQUISITION_FAILED: &str = "surface is unavailable after recovery";
const SURFACE_VALIDATION_FAILED: &str = "surface texture acquisition failed validation";
const CLEAR_COLOR: wgpu::Color = wgpu::Color::BLACK;
const ACQUISITION_ATTEMPTS: usize = 2;

/// A window-bound render surface with its device and presentation queue.
///
/// Zero-sized surfaces are suspended until resized to nonzero dimensions.
/// This initialization milestone clears and presents the surface; geometry
/// drawing is implemented separately in issue #13.
#[derive(Debug)]
pub struct RenderSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    size: WindowSize,
    configured: bool,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    instance: wgpu::Instance,
    window: Arc<NativeWindow>,
}

impl RenderSurface {
    pub(crate) fn new(window: &Window) -> Result<Self, RenderError> {
        let size = window.size();
        let native = window
            .native()
            .map_err(|error| RenderError::Initialization(error.to_string()))?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(Arc::clone(&native)),
        ));
        let surface = instance
            .create_surface(Arc::clone(&native))
            .map_err(|error| RenderError::Initialization(format!("create surface: {error}")))?;
        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
                .map_err(|error| {
                    RenderError::Initialization(format!("request adapter: {error}"))
                })?;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some(DEVICE_LABEL),
                    ..Default::default()
                })
                .await
                .map_err(|error| RenderError::Initialization(format!("request device: {error}")))?;
            Ok::<_, RenderError>((adapter, device, queue))
        })?;
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| RenderError::Initialization(NO_CONFIGURATION.to_owned()))?;
        let mut result = Self {
            surface,
            config,
            size,
            configured: false,
            adapter,
            device,
            queue,
            instance,
            window: native,
        };
        result
            .resize(size)
            .map_err(|error| RenderError::Initialization(format!("configure surface: {error}")))?;
        Ok(result)
    }

    pub(crate) fn resize(&mut self, size: WindowSize) -> Result<(), RenderError> {
        validate_surface_size(size, self.device.limits().max_texture_dimension_2d)?;
        self.size = size;
        self.configured = false;
        if is_drawable(size) {
            self.config.width = size.width;
            self.config.height = size.height;
            self.configure()
        } else {
            Ok(())
        }
    }

    fn configure(&mut self) -> Result<(), RenderError> {
        self.configured = false;
        checked_gpu_operation(&self.device, || {
            self.surface.configure(&self.device, &self.config)
        })?;
        self.configured = true;
        Ok(())
    }

    pub(crate) fn clear_and_present(&mut self) -> Result<(), RenderError> {
        if is_drawable(self.size) {
            if !self.configured {
                self.configure()?;
            }
            if let Some(output) = self.acquire()? {
                checked_gpu_operation(&self.device, || {
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder =
                        self.device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some(ENCODER_LABEL),
                            });
                    {
                        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some(CLEAR_PASS_LABEL),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                    }
                    self.queue.submit([encoder.finish()]);
                    self.window.pre_present_notify();
                    self.queue.present(output);
                })?;
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn acquire(&mut self) -> Result<Option<wgpu::SurfaceTexture>, RenderError> {
        for _ in 0..ACQUISITION_ATTEMPTS {
            match checked_gpu_operation(&self.device, || self.surface.get_current_texture())? {
                wgpu::CurrentSurfaceTexture::Success(texture) => return Ok(Some(texture)),
                wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                    self.configured = false;
                    return Ok(Some(texture));
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return Ok(None);
                }
                wgpu::CurrentSurfaceTexture::Outdated => self.configure()?,
                wgpu::CurrentSurfaceTexture::Lost => {
                    self.surface = self
                        .instance
                        .create_surface(Arc::clone(&self.window))
                        .map_err(|error| {
                            RenderError::Render(format!("recreate surface: {error}"))
                        })?;
                    self.config = self
                        .surface
                        .get_default_config(&self.adapter, self.size.width, self.size.height)
                        .ok_or_else(|| RenderError::Render(NO_CONFIGURATION.to_owned()))?;
                    self.configure()?;
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(RenderError::Render(SURFACE_VALIDATION_FAILED.to_owned()));
                }
            }
        }
        Err(RenderError::Render(ACQUISITION_FAILED.to_owned()))
    }
}

fn is_drawable(size: WindowSize) -> bool {
    size.width != 0 && size.height != 0
}

fn validate_surface_size(size: WindowSize, limit: u32) -> Result<(), RenderError> {
    if size.width > limit || size.height > limit {
        Err(RenderError::Render(format!(
            "surface size {}x{} exceeds device limit {limit}",
            size.width, size.height
        )))
    } else {
        Ok(())
    }
}

fn checked_gpu_operation<T>(
    device: &wgpu::Device,
    operation: impl FnOnce() -> T,
) -> Result<T, RenderError> {
    let scopes = [
        wgpu::ErrorFilter::OutOfMemory,
        wgpu::ErrorFilter::Internal,
        wgpu::ErrorFilter::Validation,
    ]
    .map(|filter| device.push_error_scope(filter));
    let result = operation();
    let errors = scopes
        .into_iter()
        .rev()
        .filter_map(|scope| pollster::block_on(scope.pop()))
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(result)
    } else {
        Err(RenderError::Render(errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimized_surface_is_not_drawable() {
        assert!(!is_drawable(WindowSize::new(0, 600)));
        assert!(!is_drawable(WindowSize::new(800, 0)));
    }

    #[test]
    fn surface_size_at_device_limit_is_accepted() {
        assert!(validate_surface_size(WindowSize::new(2048, 2048), 2048).is_ok());
    }

    #[test]
    fn oversized_surface_is_rejected() {
        assert!(matches!(
            validate_surface_size(WindowSize::new(2049, 600), 2048),
            Err(RenderError::Render(_))
        ));
        assert!(matches!(
            validate_surface_size(WindowSize::new(800, 2049), 2048),
            Err(RenderError::Render(_))
        ));
    }
}
