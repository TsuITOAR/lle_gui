struct Uniforms {
    width: u32,
    height: u32,
    z_range: vec2<f32>,
    compute_mode: u32,
    rf_db_scale: u32,
    _padding: vec2<u32>,
};

@group(0) @binding(0)
var<storage, read> raw_data: array<f32>;

@group(0) @binding(1)
var<storage, read> rf_input: array<vec2<f32>>;

@group(0) @binding(2)
var<storage, read_write> rf_fft_state: array<vec2<f32>>;

@group(0) @binding(3)
var<storage, read_write> rf_values: array<f32>;

@group(0) @binding(4)
var<storage, read_write> rf_bin_minmax: array<vec2<f32>>;

@group(0) @binding(5)
var<storage, read_write> cache_data: array<u32>;

@group(0) @binding(6)
var<uniform> uniforms: Uniforms;

@group(0) @binding(7)
var<storage, read> colormap: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn main_raw(@builtin(global_invocation_id) global_id: vec3<u32>) {
    compute_raw(global_id);
}

@compute @workgroup_size(FFT_WG_X, FFT_WG_BINS, 1)
fn main_rf_stage1(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    compute_rf_fft_stage1(global_id, local_id, workgroup_id);
}

@compute @workgroup_size(8, 8, 1)
fn main_rf_stage2(@builtin(global_invocation_id) global_id: vec3<u32>) {
    compute_rf_fft_stage2(global_id);
}

fn compute_raw(global_id: vec3<u32>) {
    if global_id.x >= uniforms.width || global_id.y >= uniforms.height {
        return;
    }
    let index = global_id.x + global_id.y * uniforms.width;
    let denom = max(uniforms.z_range[1] - uniforms.z_range[0], 1e-12);
    let value = (raw_data[index] - uniforms.z_range[0]) / denom;
    cache_data[index] = sample_colormap(value);
}

fn compute_rf_fft_stage1(
    _global_id: vec3<u32>,
    local_id: vec3<u32>,
    workgroup_id: vec3<u32>,
) {
    let bin = workgroup_id.x * FFT_WG_BINS + local_id.y;
    let n = uniforms.height;
    let is_active = bin < uniforms.width && n >= 2u;

    let use_shared = n <= FFT_SHARED_MAX_N;
    if use_shared {
        fft_impl_shared(bin, n, local_id.x, local_id.y, is_active);
    } else {
        fft_impl_storage(bin, n, local_id.x, is_active);
    }

    if is_active && local_id.x == 0u {
        let split_pos = (n + 1u) / 2u;
        var col_min = 1e30;
        var col_max = -1e30;
        for (var rf_idx = 0u; rf_idx < n; rf_idx = rf_idx + 1u) {
            let k = (rf_idx + split_pos) % n;
            var fft_value = vec2<f32>(0.0, 0.0);
            if use_shared {
                fft_value = wg_fft[local_id.y * FFT_SHARED_MAX_N + k];
            } else {
                fft_value = rf_fft_state[k * uniforms.width + bin];
            }
            let v = magnitude_with_db(fft_value, uniforms.rf_db_scale);
            let out_index = rf_idx * uniforms.width + bin;
            rf_values[out_index] = v;
            if is_finite_f32(v) {
                col_min = min(col_min, v);
                col_max = max(col_max, v);
            }
        }
        if col_min >= col_max{
            col_min = 0.0;
            col_max = 1.0;

        }
        else if !is_finite_f32(col_min) {
            col_min = 0.0;
        }else if  !is_finite_f32(col_max) {
            col_max = 1.0;
        }
        rf_bin_minmax[bin] = vec2<f32>(col_min, col_max);
    }
}

fn compute_rf_fft_stage2(global_id: vec3<u32>) {
    if global_id.x >= uniforms.width || global_id.y >= uniforms.height {
        return;
    }
    let bin = global_id.x;
    let rf_idx = global_id.y;
    let out_index = rf_idx * uniforms.width + bin;
    var v = rf_values[out_index];
    let mm = rf_bin_minmax[bin];
    let span = mm.y - mm.x;
    if is_finite_f32(v) && is_finite_f32(span) && span > 0.0 {
        v = (v - mm.x) / span;
    } else {
        v = 0.0;
    }
    cache_data[out_index] = sample_colormap(v);
}


fn sample_colormap(value: f32) -> u32 {
    let colormap_size = arrayLength(&colormap);
    let value_clamped = clamp(value, 0.0, 1.0);
    let scaled_value = value_clamped * f32(colormap_size - 1);
    let index = u32(round(scaled_value));

    return colormap[index];
}
