use iterator_ilp::IteratorILP;

use super::Drawer;

use crate::{
    FftSource,
    drawer::{DrawMat, Process},
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
        self.set_raw_mode();
        let mut log = self.data();
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
                .map(|d| proc.clone().proc_f32(d, true))
                .zip(log.par_rchunks_exact_mut(chunk_size).into_par_iter())
                .for_each(|(src, dst)| {
                    dst.clone_from_slice(&src);
                })
        }

        let (max, min) = search_max_min(&log);

        drop(log);
        self.set_z_range([min, max]);
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
        if global_norm {
            return false;
        }
        let max_log = self
            .max_log()
            .map(|x| x.get())
            .unwrap_or(history_data.len());
        let mut time_len = history_data.len().min(max_log);
        if time_len < 2 {
            return true;
        }
        // Shader FFT is radix-2 only; use the largest power-of-two history window.
        time_len = 1usize << (usize::BITS - 1 - time_len.leading_zeros());
        if time_len < 2 {
            return true;
        }

        let start = history_data.len().saturating_sub(time_len);
        let history_slice = &history_data[start..];
        let mut rf_input = vec![[0.0f32, 0.0f32]; time_len * chunk_size];
        for (t, d) in history_slice.iter().enumerate() {
            let row = proc.proc_complex(d, true);
            if row.len() != chunk_size {
                return false;
            }
            for (bin, v) in row.into_iter().enumerate() {
                rf_input[bin * time_len + t] = [v.re as f32, v.im as f32];
            }
        }
        self.set_rf_fft_input(chunk_size, time_len, &rf_input, proc.core.db_scale);
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

    fn set_y_label(&mut self, label: Option<String>) {
        self.axis_drawer.y_label = label;
    }
    fn set_matrix(&mut self, width: usize, height: usize, data: &[f32], z_range: Option<[f32; 2]>) {
        self.set_raw_mode();
        debug_assert_eq!(self.uniforms().width as usize, width);
        self.set_height(height as u32);
        let mut log = self.data();
        if log.len() != data.len() {
            log.resize(data.len(), 0.0);
        }
        log.copy_from_slice(data);
        let range = z_range.unwrap_or_else(|| search_max_min(&log).into());
        drop(log);
        self.set_z_range(range);
    }
}

fn search_max_min(data: &[f32]) -> (f32, f32) {
    puffin_egui::puffin::profile_function!();
    debug_assert!(data.len().is_multiple_of(2));
    data.chunks(2)
        .map(|x| (x[0], x[1]))
        .reduce_ilp::<8>(|(a, b), (c, d)| (a.max(c).max(d), b.min(c).min(d)))
        .unwrap()
    /* let (max, min) = log
    .chunks(chunk_size)
    .map(|x| {
        x.iter()
            .fold((0f32, 1f32), |(a, b), &c| (a.max(c), b.min(c)))
    })
    .fold((0f32, 1f32), |(a, b), (c, d)| (a.max(c), b.min(d))); */
}
