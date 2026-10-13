// Simple v1 mesh shader.
//
// There is no lighting in v1, so the model color is modulated with the
// vertex color to keep the mesh relief readable.
//
// Uniform layout:
//   @group(0) @binding(0)  CameraUniform { view_proj: mat4x4<f32> }
//   @group(1) @binding(0)  MeshUniform   { world: mat4x4<f32>, color: vec4<f32> }
// Vertex layout:
//   @location(0) vec3<f32> position
//   @location(1) vec3<f32> color
//
// `MeshUniform.color` is the model's linear RGBA color; the default is a
// light neutral color. The vertex color is modulated with it.

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct MeshUniform {
    world: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> mesh: MeshUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_position = mesh.world * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_position;

    // Vertex RGB color, tinted by the mesh's uniform color.
    out.color = vec4<f32>(mesh.color.rgb * in.color, mesh.color.a);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}