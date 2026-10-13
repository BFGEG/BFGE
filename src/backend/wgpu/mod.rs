//! wgpu-based rendering backend.

pub mod backend;
pub mod buffer;
pub mod pipeline;
pub mod surface;
pub mod texture;

fn checked_gpu_operation<T>(
    device: &wgpu::Device,
    operation: impl FnOnce() -> T,
) -> Result<T, crate::render::system::RenderError> {
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
        Err(crate::render::system::RenderError::Render(
            errors.join("; "),
        ))
    }
}
