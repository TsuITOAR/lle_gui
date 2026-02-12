override FFT_WG_X: u32 = 64u;
override FFT_WG_BINS: u32 = 4u;
override FFT_SHARED_MAX_N: u32 = 512u;

var<workgroup> wg_fft: array<vec2<f32>, FFT_WG_BINS * FFT_SHARED_MAX_N>;

fn complex_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

fn complex_sub(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x - b.x, a.y - b.y);
}

fn is_finite_f32(v: f32) -> bool {
    // NaN fails self-equality; Inf exceeds finite f32 max magnitude.
    return v == v && abs(v) <= 3.4028235e38;
}

fn ilog2_pow2(v: u32) -> u32 {
    var x = v;
    var bits = 0u;
    while x > 1u {
        x = x >> 1u;
        bits = bits + 1u;
    }
    return bits;
}

fn reverse_bits_width(v: u32, width: u32) -> u32 {
    return reverseBits(v) >> (32u - width);
}

fn magnitude_with_db(c: vec2<f32>, db_scale: u32) -> f32 {
    // Overflow-safe magnitude: avoid squaring very large values directly.
    let ax = abs(c.x);
    let ay = abs(c.y);
    let m = max(ax, ay);
    var v = 0.0;
    if m > 0.0 {
        let sx = c.x / m;
        let sy = c.y / m;
        v = m * sqrt(sx * sx + sy * sy);
    }
    if db_scale != 0u {
        if v > 0.0 {
            v = 20.0 * log(v) / log(10.0);
        } else {
            v = -40.0;
        }
    }
    return v;
}

fn fft_impl_storage(bin: u32, n: u32, local_x: u32, is_active: bool) {
    let bits = ilog2_pow2(n);
    // Cooperative bit-reversal load into storage FFT buffer.
    for (var i = local_x; i < n; i = i + FFT_WG_X) {
        if is_active {
            let j = reverse_bits_width(i, bits);
            let dst = i * uniforms.width + bin;
            let src = bin * n + j;
            rf_fft_state[dst] = rf_input[src];
        }
    }
    storageBarrier();
    workgroupBarrier();

    let pi = 3.141592653589793;
    var len = 2u;
    while len <= n {
        let half = len / 2u;
        let theta = -2.0 * pi / f32(len);
        let total_pairs = n / 2u;
        for (var pair = local_x; pair < total_pairs; pair = pair + FFT_WG_X) {
            let segment = pair / half;
            let j = pair % half;
            let i0 = segment * len + j;
            let i1 = i0 + half;
            if is_active {
                let idx0 = i0 * uniforms.width + bin;
                let idx1 = i1 * uniforms.width + bin;
                let w = vec2<f32>(cos(theta * f32(j)), sin(theta * f32(j)));
                let a = rf_fft_state[idx0];
                let b = complex_mul(rf_fft_state[idx1], w);
                rf_fft_state[idx0] = a + b;
                rf_fft_state[idx1] = complex_sub(a, b);
            }
        }
        storageBarrier();
        workgroupBarrier();
        len = len << 1u;
    }
}

fn fft_impl_shared(bin: u32, n: u32, local_x: u32, local_y: u32, is_active: bool) {
    let bits = ilog2_pow2(n);
    let base = local_y * FFT_SHARED_MAX_N;
    for (var i = local_x; i < n; i = i + FFT_WG_X) {
        if is_active {
            let j = reverse_bits_width(i, bits);
            wg_fft[base + i] = rf_input[bin * n + j];
        } else {
            wg_fft[base + i] = vec2<f32>(0.0, 0.0);
        }
    }
    workgroupBarrier();

    let pi = 3.141592653589793;
    var len = 2u;
    while len <= n {
        let half = len / 2u;
        let theta = -2.0 * pi / f32(len);
        let total_pairs = n / 2u;
        for (var pair = local_x; pair < total_pairs; pair = pair + FFT_WG_X) {
            let segment = pair / half;
            let j = pair % half;
            let idx0 = base + segment * len + j;
            let idx1 = idx0 + half;
            let w = vec2<f32>(cos(theta * f32(j)), sin(theta * f32(j)));
            let a = wg_fft[idx0];
            let b = complex_mul(wg_fft[idx1], w);
            wg_fft[idx0] = a + b;
            wg_fft[idx1] = complex_sub(a, b);
        }
        workgroupBarrier();
        len = len << 1u;
    }
}
