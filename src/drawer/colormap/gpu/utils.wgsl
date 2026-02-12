// True when `v` is finite (not NaN/Inf).
fn is_finite_f32(v: f32) -> bool {
    // NaN fails self-equality; Inf exceeds finite f32 max magnitude.
    return v == v && abs(v) <= 3.4028235e38;
}

// Computes complex magnitude, optionally converted to dB.
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
