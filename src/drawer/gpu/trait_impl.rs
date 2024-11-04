use lle::num_complex::Complex64;

use crate::drawer::chart::{DrawMat, Process};

use super::Drawer;

impl DrawMat for Drawer {
    fn draw_mat_on_ui(&mut self, _len: usize, ui: &mut egui::Ui) -> Result<(), eframe::Error> {
        puffin::profile_function!();
        self.show(ui);
        Ok(())
    }
    fn fetch(&mut self, data: &[Complex64], proc: &mut Process, chunk_size: usize) {
        self.update(data, proc, chunk_size);
    }
    fn update(&mut self, data: &[Complex64], proc: &mut Process, chunk_size: usize) {
        puffin::profile_function!();
        let mut log = self.data();
        let max_log = self.max_log().unwrap().get();

        for (r, d) in data.rchunks(chunk_size).enumerate().take(max_log).rev() {
            let start = (max_log - 1 - r) * chunk_size;
            let end = start + chunk_size;
            log[start..end].copy_from_slice(&proc.proc_f32(d));
        }

        let (max, min) = log
            .chunks(chunk_size)
            .map(|x| {
                x.iter()
                    .fold((0f32, 1f32), |(a, b), &c| (a.max(c), b.min(c)))
            })
            .fold((0f32, 1f32), |(a, b), (c, d)| (a.max(c), b.min(d)));
        drop(log);
        self.uniforms_mut().z_range = [min, max];
    }

    fn max_log(&self) -> Option<std::num::NonZero<usize>> {
        std::num::NonZero::new(self.uniforms().height as usize)
    }

    fn set_max_log(&mut self, max_log: std::num::NonZero<usize>) {
        self.uniforms_mut().height = max_log.get() as u32;
        self.data()
            .resize((self.uniforms().height * self.uniforms().width) as _, 0.0);
    }
}
