struct CameraUniform {
    view_proj: mat4x4<f32>,
    ambient: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
};

struct MeshUniform {
    world: mat4x4<f32>,
    normal_matrix: mat3x3<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> mesh: MeshUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * mesh.world * vec4<f32>(in.position, 1.0);
    out.normal = mesh.normal_matrix * in.normal;
    out.color = mesh.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var diffuse = 0.0;
    if dot(in.normal, in.normal) > 0.0 {
        diffuse = max(dot(normalize(in.normal), -camera.light_direction.xyz), 0.0);
    }
    let illumination = camera.ambient.rgb + diffuse * camera.light_color.rgb;
    return vec4<f32>(in.color.rgb * illumination, in.color.a);
}
