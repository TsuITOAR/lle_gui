use super::Drawer;

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
        let cur_h = self.uniforms().height.max(2);
        self.set_size(chunk_size as u32, cur_h);
        let mut raw = self.data();
        let max_log = self.max_log().unwrap().get();
        use rayon::prelude::*;
        {
            puffin_egui::puffin::profile_scope!("process data");
            proc.delta.deactivate();
            data.iter()
                .rev()
                .take(max_log)
                .collect::<Vec<&S>>()
                .into_par_iter()
                .map(|d| proc.clone().proc_complex(d, true))
                .zip(raw.par_rchunks_exact_mut(chunk_size).into_par_iter())
                .for_each(|(src, dst)| {
                    for (d, s) in dst.iter_mut().zip(src.into_iter()) {
                        *d = [s.re as f32, s.im as f32];
                    }
                })
        }
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
        let Some(cached_input) = update_rf_cache(
            &self.rf_gpu_cache,
            history_data,
            &mut proc_local,
            start,
            history_len,
            time_len,
            chunk_size,
        ) else {
            return false;
        };
        self.set_rf_fft_input(
            chunk_size,
            time_len,
            &cached_input,
            proc.core.db_scale,
            global_norm,
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

    fn set_matrix(&mut self, width: usize, height: usize, data: &[f32], z_range: Option<[f32; 2]>) {
        self.set_raw_mode(Component::Real, false, z_range.is_none());
        self.set_size(width as u32, height as u32);
        let mut raw = self.data();
        if raw.len() != data.len() {
            raw.resize(data.len(), [0.0, 0.0]);
        }
        for (dst, &src) in raw.iter_mut().zip(data.iter()) {
            *dst = [src, 0.0];
        }
        drop(raw);
        if let Some(range) = z_range {
            self.set_z_range(range);
        }
    }
}

fn fill_rf_row<S: FftSource>(
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

fn fill_rf_row_zero(dst: &mut [[f32; 2]], chunk_size: usize, t: usize) {
    let row_dst = &mut dst[t * chunk_size..(t + 1) * chunk_size];
    row_dst.fill([0.0, 0.0]);
}

fn fill_rf_window<S: FftSource>(
    proc: &mut Process<S>,
    history_data: &[S],
    start: usize,
    history_len: usize,
    time_len: usize,
    chunk_size: usize,
    dst: &mut [[f32; 2]],
) -> bool
where
    S::FftProcessor: Sync,
{
    let valid = history_len.saturating_sub(start).min(time_len);
    let pad = time_len - valid;
    for t in 0..time_len {
        if t < pad {
            fill_rf_row_zero(dst, chunk_size, t);
            continue;
        }
        let abs_idx = start + (t - pad);
        if abs_idx >= history_len {
            fill_rf_row_zero(dst, chunk_size, t);
            continue;
        }
        if !fill_rf_row(proc, &history_data[abs_idx], chunk_size, dst, t) {
            return false;
        }
    }
    true
}

fn update_rf_cache<S: FftSource>(
    cache_mutex: &egui::mutex::Mutex<super::RfGpuInputCache>,
    history_data: &[S],
    proc: &mut Process<S>,
    start: usize,
    history_len: usize,
    time_len: usize,
    chunk_size: usize,
) -> Option<Vec<super::RfInputFormat>>
where
    S::FftProcessor: Sync,
{
    let mut cache = cache_mutex.lock();
    let rebuild = cache.time_len != time_len
        || cache.data.len() != time_len * chunk_size
        || cache.history_len > history_len
        || start < cache.start
        || start > cache.start.saturating_add(time_len);

    if rebuild {
        cache.start = start;
        cache.history_len = history_len;
        cache.time_len = time_len;
        cache.data.resize(time_len * chunk_size, [0.0, 0.0]);
        if !fill_rf_window(
            proc,
            history_data,
            start,
            history_len,
            time_len,
            chunk_size,
            &mut cache.data,
        ) {
            return None;
        }
    }

    let prev_start = cache.start;
    let prev_end = prev_start + time_len;
    let dropped = start.saturating_sub(prev_start);
    if dropped > 0 {
        if dropped >= time_len {
            if !fill_rf_window(
                proc,
                history_data,
                start,
                history_len,
                time_len,
                chunk_size,
                &mut cache.data,
            ) {
                return None;
            }
        } else {
            let row_span = chunk_size * dropped;
            cache.data.copy_within(row_span..time_len * chunk_size, 0);
            let tail_rows = dropped;
            let fill_start_t = time_len - tail_rows;
            for t in 0..tail_rows {
                let abs_idx = prev_end + t;
                if abs_idx >= history_len {
                    return None;
                }
                if !fill_rf_row(
                    proc,
                    &history_data[abs_idx],
                    chunk_size,
                    &mut cache.data,
                    fill_start_t + t,
                ) {
                    return None;
                }
            }
        }
    } else if history_len > cache.history_len {
        let growth = history_len - cache.history_len;
        let tail_rows = growth.min(time_len);
        let fill_start_t = time_len - tail_rows;
        for t in 0..tail_rows {
            let abs_idx = history_len - tail_rows + t;
            if !fill_rf_row(
                proc,
                &history_data[abs_idx],
                chunk_size,
                &mut cache.data,
                fill_start_t + t,
            ) {
                return None;
            }
        }
    }
    cache.start = start;
    cache.history_len = history_len;

    Some(cache.data.clone())
}
