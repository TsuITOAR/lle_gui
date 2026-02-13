use super::Drawer;
use rayon::prelude::*;

use crate::{
    FftSource,
    drawer::{DrawMat, Process, processor::Component},
};

impl DrawMat for Drawer {
    fn draw_mat_on_ui(&mut self, _len: usize, ui: &mut egui::Ui) -> Result<(), eframe::Error> {
        puffin_egui::puffin::profile_function!();
        self.show(ui);
        Ok(())
    }

    fn fetch<S: FftSource>(&mut self, data: &[S], proc: &mut Process<S>, chunk_size: usize)
    where
        S::FftProcessor: Sync,
    {
        puffin_egui::puffin::profile_function!();
        self.set_raw_mode(proc.core.component, proc.core.db_scale, true);
        let max_log = self.max_log().unwrap().get();
        self.set_size(chunk_size as u32, max_log as u32);
        let start = data.len().saturating_sub(max_log);
        let history_len = data.len();
        let mut proc_local = proc.clone();
        proc_local.delta.deactivate();
        let Some(cached_raw) = update_cache_with(
            &self.raw_gpu_cache,
            data,
            start,
            history_len,
            max_log,
            chunk_size,
            &proc_local,
        ) else {
            return;
        };
        let mut raw = self.data();
        raw.copy_from_slice(&cached_raw);
        drop(raw);
    }

    fn fetch_rf_fft_gpu<S: FftSource>(
        &mut self,
        history_data: &[S],
        proc: &mut Process<S>,
        chunk_size: usize,
        global_norm: bool,
    ) -> bool
    where
        S::FftProcessor: Sync,
    {
        puffin_egui::puffin::profile_function!();
        let max_log = self
            .max_log()
            .map(|x| x.get())
            .unwrap_or(history_data.len());
        let mut time_len = max_log;
        if time_len < 2 {
            return true;
        }
        // Shader FFT is radix-2 only; use the largest power-of-two history window.
        time_len = 1usize << (usize::BITS - 1 - time_len.leading_zeros());
        if time_len < 2 {
            return true;
        }

        let start = history_data.len().saturating_sub(time_len);
        let history_len = history_data.len();
        let mut proc_local = proc.clone();
        proc_local.delta.deactivate();
        let Some(cached_input) = update_cache_with(
            &self.rf_gpu_cache,
            history_data,
            start,
            history_len,
            time_len,
            chunk_size,
            &proc_local,
        ) else {
            return false;
        };
        self.set_gpu_input(
            chunk_size,
            time_len,
            &cached_input,
            proc.core.db_scale,
            global_norm,
            true,
        );
        true
    }

    fn max_log(&self) -> Option<std::num::NonZero<usize>> {
        std::num::NonZero::new(self.uniforms().height as usize)
    }

    fn set_max_log(&mut self, max_log: std::num::NonZero<usize>) {
        self.set_height(max_log.get() as u32);
    }

    fn set_align_x_axis(&mut self, align: impl Into<Option<(f32, f32)>>) {
        self.axis_drawer.align_x_axis = align.into();
    }

    // fn set_y_label(&mut self, label: Option<String>) {
    //     self.axis_drawer.y_label = label;
    // }

    fn set_y_tick_shift(&mut self, shift: i32) {
        self.axis_drawer.y_tick_shift = shift;
    }

    fn set_matrix(
        &mut self,
        width: usize,
        height: usize,
        data: &[f32],
        _z_range: Option<[f32; 2]>,
    ) {
        self.set_raw_mode(Component::Real, false, true);
        self.set_size(width as u32, height as u32);
        let mut raw = self.data();
        if raw.len() != data.len() {
            raw.resize(data.len(), [0.0, 0.0]);
        }
        for (dst, &src) in raw.iter_mut().zip(data.iter()) {
            *dst = [src, 0.0];
        }
        drop(raw);
    }
}

fn fill_complex_row<S: FftSource>(
    proc: &mut Process<S>,
    src: &S,
    chunk_size: usize,
    dst: &mut [[f32; 2]],
    t: usize,
) -> bool
where
    S::FftProcessor: Sync,
{
    let row = proc.proc_complex(src, true);
    if row.len() != chunk_size {
        return false;
    }
    let row_dst = &mut dst[t * chunk_size..(t + 1) * chunk_size];
    for (bin, v) in row.into_iter().enumerate() {
        row_dst[bin] = [v.re as f32, v.im as f32];
    }
    true
}

fn fill_zero_row(dst: &mut [[f32; 2]], chunk_size: usize, t: usize) {
    let row_dst = &mut dst[t * chunk_size..(t + 1) * chunk_size];
    row_dst.fill([0.0, 0.0]);
}

fn fill_window_with_parallel<S>(
    history_data: &[S],
    start: usize,
    history_len: usize,
    time_len: usize,
    chunk_size: usize,
    dst: &mut [[f32; 2]],
    proc_template: &Process<S>,
) -> bool
where
    S: FftSource + Sync,
    S::FftProcessor: Sync,
{
    let valid = history_len.saturating_sub(start).min(time_len);
    let pad = time_len - valid;
    dst.par_chunks_mut(chunk_size)
        .enumerate()
        .try_for_each_init(
            || proc_template.clone(),
            |proc_local, (t, row_dst)| {
                if t < pad {
                    row_dst.fill([0.0, 0.0]);
                    return Ok(());
                }
                let abs_idx = start + (t - pad);
                if abs_idx >= history_len {
                    row_dst.fill([0.0, 0.0]);
                    return Ok(());
                }
                let row = proc_local.proc_complex(&history_data[abs_idx], true);
                if row.len() != chunk_size {
                    return Err(());
                }
                for (bin, v) in row.into_iter().enumerate() {
                    row_dst[bin] = [v.re as f32, v.im as f32];
                }
                Ok(())
            },
        )
        .is_ok()
}

fn rebuild_cache<S>(
    cache: &mut super::GpuInputCache,
    history_data: &[S],
    start: usize,
    history_len: usize,
    time_len: usize,
    chunk_size: usize,
    proc_template: &Process<S>,
) -> bool
where
    S: FftSource + Sync,
    S::FftProcessor: Sync,
{
    cache.start = start;
    cache.history_len = history_len;
    cache.time_len = time_len;
    cache.data.resize(time_len * chunk_size, [0.0, 0.0]);
    fill_window_with_parallel(
        history_data,
        start,
        history_len,
        time_len,
        chunk_size,
        &mut cache.data,
        proc_template,
    )
}

fn refill_full_window<S>(
    cache_data: &mut [[f32; 2]],
    history_data: &[S],
    start: usize,
    history_len: usize,
    time_len: usize,
    chunk_size: usize,
    proc_local: &mut Process<S>,
) -> bool
where
    S: FftSource,
    S::FftProcessor: Sync,
{
    let valid = history_len.saturating_sub(start).min(time_len);
    let pad = time_len - valid;
    for t in 0..time_len {
        if t < pad {
            fill_zero_row(cache_data, chunk_size, t);
            continue;
        }
        let abs_idx = start + (t - pad);
        if abs_idx >= history_len {
            fill_zero_row(cache_data, chunk_size, t);
            continue;
        }
        if !fill_complex_row(
            proc_local,
            &history_data[abs_idx],
            chunk_size,
            cache_data,
            t,
        ) {
            return false;
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn advance_window_partial<S>(
    cache_data: &mut [[f32; 2]],
    history_data: &[S],
    prev_end: usize,
    history_len: usize,
    time_len: usize,
    dropped: usize,
    chunk_size: usize,
    proc_local: &mut Process<S>,
) -> bool
where
    S: FftSource,
    S::FftProcessor: Sync,
{
    let row_span = chunk_size * dropped;
    cache_data.copy_within(row_span..time_len * chunk_size, 0);
    let fill_start_t = time_len - dropped;
    for t in 0..dropped {
        let abs_idx = prev_end + t;
        if abs_idx >= history_len {
            return false;
        }
        if !fill_complex_row(
            proc_local,
            &history_data[abs_idx],
            chunk_size,
            cache_data,
            fill_start_t + t,
        ) {
            return false;
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn grow_window_tail<S>(
    cache_data: &mut [[f32; 2]],
    history_data: &[S],
    start: usize,
    prev_history_len: usize,
    history_len: usize,
    time_len: usize,
    chunk_size: usize,
    proc_local: &mut Process<S>,
) -> bool
where
    S: FftSource,
    S::FftProcessor: Sync,
{
    let prev_valid = prev_history_len.saturating_sub(start).min(time_len);
    let new_valid = history_len.saturating_sub(start).min(time_len);
    if new_valid <= prev_valid {
        return true;
    }

    let growth = new_valid - prev_valid;
    // Keep the existing valid tail window aligned when history is still shorter than time_len.
    if prev_valid > 0 {
        let old_pad = time_len - prev_valid;
        let new_pad = time_len - new_valid;
        let src_begin = old_pad * chunk_size;
        let src_end = src_begin + prev_valid * chunk_size;
        let dst_begin = new_pad * chunk_size;
        cache_data.copy_within(src_begin..src_end, dst_begin);
    }

    let fill_start_t = time_len - growth;
    for t in 0..growth {
        let abs_idx = history_len - growth + t;
        if !fill_complex_row(
            proc_local,
            &history_data[abs_idx],
            chunk_size,
            cache_data,
            fill_start_t + t,
        ) {
            return false;
        }
    }
    true
}

fn update_cache_with<S>(
    cache_mutex: &egui::mutex::Mutex<super::GpuInputCache>,
    history_data: &[S],
    start: usize,
    history_len: usize,
    time_len: usize,
    chunk_size: usize,
    proc_template: &Process<S>,
) -> Option<Vec<[f32; 2]>>
where
    S: FftSource + Sync,
    S::FftProcessor: Sync,
{
    puffin_egui::puffin::profile_function!();
    let mut cache = cache_mutex.lock();
    let rebuild = cache.time_len != time_len
        || cache.data.len() != time_len * chunk_size
        || cache.history_len > history_len
        || start < cache.start;

    if rebuild {
        if !rebuild_cache(
            &mut cache,
            history_data,
            start,
            history_len,
            time_len,
            chunk_size,
            proc_template,
        ) {
            return None;
        }
        return Some(cache.data.clone());
    }

    let mut proc_local = proc_template.clone();
    let prev_start = cache.start;
    let prev_end = prev_start + time_len;
    let prev_history_len = cache.history_len;
    let dropped = start.saturating_sub(prev_start);
    if dropped >= time_len {
        if !refill_full_window(
            &mut cache.data,
            history_data,
            start,
            history_len,
            time_len,
            chunk_size,
            &mut proc_local,
        ) {
            return None;
        }
    } else if dropped > 0 {
        if !advance_window_partial(
            &mut cache.data,
            history_data,
            prev_end,
            history_len,
            time_len,
            dropped,
            chunk_size,
            &mut proc_local,
        ) {
            return None;
        }
    } else if history_len > prev_history_len
        && !grow_window_tail(
            &mut cache.data,
            history_data,
            start,
            prev_history_len,
            history_len,
            time_len,
            chunk_size,
            &mut proc_local,
        )
    {
        return None;
    }

    cache.start = start;
    cache.history_len = history_len;

    Some(cache.data.clone())
}
