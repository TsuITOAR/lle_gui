struct Uniforms {
    width: u32,
    height: u32,
    z_range: vec2<f32>,
};

@group(0) @binding(0)
var<storage, read> raw_data: array<f32>;

@group(0) @binding(1)
var<storage, read_write> cache_data: array<u32>;

@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

@group(0) @binding(3)
var<storage, read> colormap: array<u32>;

@compute @workgroup_size(16, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x + global_id.y * uniforms.width;
    let value = (raw_data[index] - uniforms.z_range[0]) / (uniforms.z_range[1] - uniforms.z_range[0]);
    cache_data[index] = sample_colormap(value);
}


fn sample_colormap(value: f32) -> u32 {
    let colormap_size = arrayLength(&colormap);
    let scaled_value = value * f32(colormap_size - 1);
    let index = u32(round(scaled_value));

    return colormap[index];
}