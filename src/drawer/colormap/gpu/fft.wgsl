override FFT_WG_X: u32 = 64u;
override FFT_WG_BINS: u32 = 4u;
override FFT_SHARED_MAX_N: u32 = 512u;

var<workgroup> wg_fft: array<vec2<f32>, FFT_WG_BINS * FFT_SHARED_MAX_N>;
const PI: f32 = 3.141592653589793;

// ---------- math helpers ----------
// Complex multiply for values encoded as vec2(real, imag).
fn complex_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

// Returns e^{i * theta * j} as a complex twiddle factor.
fn twiddle(theta: f32, j: u32) -> vec2<f32> {
    return vec2<f32>(cos(theta * f32(j)), sin(theta * f32(j)));
}

// ---------- integer helpers ----------
// Computes log2(v) for power-of-two v.
fn ilog2_pow2(v: u32) -> u32 {
    var x = v;
    var bits = 0u;
    while x > 1u {
        x = x >> 1u;
        bits = bits + 1u;
    }
    return bits;
}

// Reverses the lower `width` bits in `v`.
fn reverse_bits_width(v: u32, width: u32) -> u32 {
    return reverseBits(v) >> (32u - width);
}

// Storage layout helpers:
// - rf_input: column-major time x bin, index = t * width + bin
// - rf_fft_state: bin-major, index = bin * n + t
// Linear index helper for time-major (column-major by bin) layout.
fn idx_col_major_time_bin(t: u32, bin: u32, width: u32) -> u32 {
    return t * width + bin;
}

// Linear index helper for bin-major layout.
fn idx_bin_major_time(bin: u32, t: u32, n: u32) -> u32 {
    return bin * n + t;
}

// One radix-2 butterfly step in storage-buffer scratch.
fn radix2_storage(bin: u32, j: u32, len: u32, segment: u32) {
    let half = len / 2u;
    let i0 = segment * len + j;
    let i1 = i0 + half;
    let idx0 = idx_col_major_time_bin(i0, bin, uniforms.width);
    let idx1 = idx_col_major_time_bin(i1, bin, uniforms.width);
    let theta = -2.0 * PI / f32(len);
    let w = twiddle(theta, j);
    let a = rf_input[idx0];
    let b = complex_mul(rf_input[idx1], w);
    rf_input[idx0] = a + b;
    rf_input[idx1] = a - b;
}

// One radix-2 butterfly step in workgroup shared memory.
fn radix2_shared(base: u32, j: u32, len: u32, segment: u32) {
    let half = len / 2u;
    let idx0 = base + segment * len + j;
    let idx1 = idx0 + half;
    let theta = -2.0 * PI / f32(len);
    let w = twiddle(theta, j);
    let a = wg_fft[idx0];
    let b = complex_mul(wg_fft[idx1], w);
    wg_fft[idx0] = a + b;
    wg_fft[idx1] = a - b;
}

// ---------- FFT implementation (storage path) ----------
// Runs iterative radix-2 FFT for one bin using storage-buffer scratch.
fn fft_impl_storage(bin: u32, n: u32, local_x: u32, is_active: bool) {
    let bits = ilog2_pow2(n);
    // Bit-reversal load from bin-major source into column-major scratch.
    for (var i = local_x; i < n; i = i + FFT_WG_X) {
        if is_active {
            let j = reverse_bits_width(i, bits);
            let dst = idx_col_major_time_bin(i, bin, uniforms.width);
            let src = idx_bin_major_time(bin, j, n);
            rf_input[dst] = rf_fft_state[src];
        }
    }
    storageBarrier();
    workgroupBarrier();

    var len = 2u;
    while len <= n {
        let half = len / 2u;
        let total_pairs = n / 2u;
        for (var pair = local_x; pair < total_pairs; pair = pair + FFT_WG_X) {
            let segment = pair / half;
            let j = pair % half;
            if is_active {
                radix2_storage(bin, j, len, segment);
            }
        }
        storageBarrier();
        workgroupBarrier();
        len = len << 1u;
    }
}

// ---------- FFT implementation (shared-memory path) ----------
// Runs iterative radix-2 FFT for one bin using workgroup shared memory.
fn fft_impl_shared(bin: u32, n: u32, local_x: u32, local_y: u32, is_active: bool) {
    let bits = ilog2_pow2(n);
    let base = local_y * FFT_SHARED_MAX_N;
    for (var i = local_x; i < n; i = i + FFT_WG_X) {
        if is_active {
            let j = reverse_bits_width(i, bits);
            wg_fft[base + i] = rf_fft_state[idx_bin_major_time(bin, j, n)];
        } else {
            wg_fft[base + i] = vec2<f32>(0.0, 0.0);
        }
    }
    workgroupBarrier();

    var len = 2u;
    while len <= n {
        let half = len / 2u;
        let total_pairs = n / 2u;
        for (var pair = local_x; pair < total_pairs; pair = pair + FFT_WG_X) {
            let segment = pair / half;
            let j = pair % half;
            radix2_shared(base, j, len, segment);
        }
        workgroupBarrier();
        len = len << 1u;
    }
}
