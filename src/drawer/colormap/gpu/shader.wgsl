// shader.wgsl

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};
// Vertex shader: pass fullscreen triangle position and derived UV.
@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.tex_coords = model.position.xy * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(0)@binding(1)
var s_diffuse: sampler;


// Fragment shader: sample rendered compute texture.
@fragment
fn fs_main(@location(0) tex_coords: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, tex_coords);
}
