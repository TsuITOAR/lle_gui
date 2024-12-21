use iterator_ilp::IteratorILP;
use lle::num_complex::Complex64;

use crate::drawer::{chart::DrawMat, Process};

use super::Drawer;

impl DrawMat for Drawer {
    fn draw_mat_on_ui(&mut self, _len: usize, ui: &mut egui::Ui) -> Result<(), eframe::Error> {
        puffin_egui::puffin::profile_function!();
        self.show(ui);
        Ok(())
    }
    fn fetch(&mut self, data: &[Complex64], proc: &mut Process, chunk_size: usize) {
        self.update(data, proc, chunk_size);
    }
    fn update(&mut self, data: &[Complex64], proc: &mut Process, chunk_size: usize) {
        puffin_egui::puffin::profile_function!();
        let mut log = self.data();
        let max_log = self.max_log().unwrap().get();
        use rayon::prelude::*;
        {
            puffin_egui::puffin::profile_scope!("process data");
            data.rchunks(chunk_size)
                .take(max_log)
                .collect::<Vec<&[Complex64]>>()
                .into_par_iter()
                .map(|d| proc.clone().proc_f32(d))
                .zip(log.par_rchunks_exact_mut(chunk_size).into_par_iter())
                .for_each(|(src, dst)| {
                    dst.clone_from_slice(&src);
                })
        }

        let (max, min) = search_max_min(&log);

        drop(log);
        self.set_z_ragne([min, max]);
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
}

fn search_max_min(data: &[f32]) -> (f32, f32) {
    puffin_egui::puffin::profile_function!();
    debug_assert!(data.len() % 2 == 0);
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
