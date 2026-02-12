struct Uniforms {
    width: u32,
    height: u32,
    z_range: vec2<f32>,
    compute_mode: u32,
    rf_db_scale: u32,
    rf_global_norm: u32,
    raw_component: u32,
    raw_db_scale: u32,
    raw_gpu_range: u32,
    _padding: vec2<u32>,
};

@group(0) @binding(0)
var<storage, read> raw_data: array<vec2<f32>>;

@group(0) @binding(1)
var<storage, read_write> rf_input: array<vec2<f32>>;

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

// Entry point for raw colormap rendering.
@compute @workgroup_size(8, 8, 1)
fn main_raw(@builtin(global_invocation_id) global_id: vec3<u32>) {
    compute_raw(global_id);
}

// Entry point for global min/max reduction in raw mode.
@compute @workgroup_size(1, 1, 1)
fn main_raw_reduce() {
    compute_raw_reduce();
}

// Entry point for transposing RF input into bin-major layout.
@compute @workgroup_size(8, 8, 1)
fn main_rf_transpose(@builtin(global_invocation_id) global_id: vec3<u32>) {
    compute_rf_transpose(global_id);
}

// Entry point for per-bin history FFT + per-bin min/max.
@compute @workgroup_size(FFT_WG_X, FFT_WG_BINS, 1)
fn main_rf_stage1(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    compute_rf_fft_stage1(global_id, local_id, workgroup_id);
}

// Entry point for optional global min/max reduction across bins.
@compute @workgroup_size(1, 1, 1)
fn main_rf_reduce_global() {
    if uniforms.width == 0u {
        return;
    }
    var global_min = 1e30;
    var global_max = -1e30;
    for (var bin = 0u; bin < uniforms.width; bin = bin + 1u) {
        let mm = rf_bin_minmax[bin];
        if is_finite_f32(mm.x) {
            global_min = min(global_min, mm.x);
        }
        if is_finite_f32(mm.y) {
            global_max = max(global_max, mm.y);
        }
    }
    if !is_finite_f32(global_min) {
        global_min = 0.0;
    }
    if !is_finite_f32(global_max) {
        global_max = 1.0;
    }
    if global_min >= global_max {
        global_min = 0.0;
        global_max = 1.0;
    }
    rf_bin_minmax[0] = vec2<f32>(global_min, global_max);
}

// Entry point for RF normalization and colormap mapping.
@compute @workgroup_size(8, 8, 1)
fn main_rf_stage2(@builtin(global_invocation_id) global_id: vec3<u32>) {
    compute_rf_fft_stage2(global_id);
}

// Converts raw complex input into scalar values and maps them to colormap indices.
fn compute_raw(global_id: vec3<u32>) {
    if global_id.x >= uniforms.width || global_id.y >= uniforms.height {
        return;
    }
    let index = global_id.x + global_id.y * uniforms.width;
    var mm = select(uniforms.z_range, rf_bin_minmax[0], uniforms.raw_gpu_range != 0u);
    if !is_finite_f32(mm.x) {
        mm.x = 0.0;
    }
    if !is_finite_f32(mm.y) {
        mm.y = 1.0;
    }
    if mm.x >= mm.y {
        mm = vec2<f32>(0.0, 1.0);
    }
    let denom = max(mm.y - mm.x, 1e-12);
    var src = raw_data[index];
    var value = 0.0;
    if uniforms.raw_component == 0u {
        value = src.x;
    } else if uniforms.raw_component == 1u {
        value = src.y;
    } else if uniforms.raw_component == 2u {
        value = length(src);
    } else {
        value = atan2(src.y, src.x);
    }
    if uniforms.raw_db_scale != 0u {
        value = 20.0 * log(value) / log(10.0);
    }
    value = (value - mm.x) / denom;
    cache_data[index] = sample_colormap(value);
}

// Computes global min/max for raw mode and stores result for normalization.
fn compute_raw_reduce() {
    var raw_min = 1e30;
    var raw_max = -1e30;
    let total = uniforms.width * uniforms.height;
    for (var i = 0u; i < total; i = i + 1u) {
        let src = raw_data[i];
        var value = 0.0;
        if uniforms.raw_component == 0u {
            value = src.x;
        } else if uniforms.raw_component == 1u {
            value = src.y;
        } else if uniforms.raw_component == 2u {
            value = length(src);
        } else {
            value = atan2(src.y, src.x);
        }
        if uniforms.raw_db_scale != 0u {
            value = 20.0 * log(value) / log(10.0);
        }
        if is_finite_f32(value) {
            raw_min = min(raw_min, value);
            raw_max = max(raw_max, value);
        }
    }
    if !is_finite_f32(raw_min) || !is_finite_f32(raw_max) || raw_min >= raw_max {
        raw_min = 0.0;
        raw_max = 1.0;
    }
    rf_bin_minmax[0] = vec2<f32>(raw_min, raw_max);
}

// Runs history-axis FFT for each bin and computes per-bin output min/max.
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
                fft_value = rf_input[k * uniforms.width + bin];
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

// Transposes RF input from time-major to bin-major memory layout.
fn compute_rf_transpose(global_id: vec3<u32>) {
    if global_id.x >= uniforms.width || global_id.y >= uniforms.height {
        return;
    }
    let bin = global_id.x;
    let t = global_id.y;
    rf_fft_state[bin * uniforms.height + t] = rf_input[t * uniforms.width + bin];
}

// Applies RF normalization (per-bin or global) and writes final colors.
fn compute_rf_fft_stage2(global_id: vec3<u32>) {
    if global_id.x >= uniforms.width || global_id.y >= uniforms.height {
        return;
    }
    let bin = global_id.x;
    let rf_idx = global_id.y;
    let out_index = rf_idx * uniforms.width + bin;
    var v = rf_values[out_index];
    let mm = select(rf_bin_minmax[bin], rf_bin_minmax[0], uniforms.rf_global_norm != 0u);
    var mm_min = mm.x;
    var mm_max = mm.y;
    if !is_finite_f32(mm_min) {
        mm_min = 0.0;
    }
    if !is_finite_f32(mm_max) {
        mm_max = 1.0;
    }
    let span = mm_max - mm_min;
    if is_finite_f32(v) && span > 0.0 {
        v = (v - mm_min) / span;
    } else {
        v = 0.0;
    }
    cache_data[out_index] = sample_colormap(v);
}


// Samples the packed u32 colormap by normalized scalar input.
fn sample_colormap(value: f32) -> u32 {
    let colormap_size = arrayLength(&colormap);
    let value_clamped = clamp(value, 0.0, 1.0);
    let scaled_value = value_clamped * f32(colormap_size - 1);
    let index = u32(round(scaled_value));

    return colormap[index];
}
