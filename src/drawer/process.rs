use std::iter::Map;

use lle::{num_complex::ComplexFloat, rustfft::FftPlanner};
use num_traits::Zero;

use super::*;

type Fft = std::sync::Arc<dyn lle::rustfft::Fft<f64>>;

#[derive(Default, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Process {
    pub(crate) fft: Option<FftProcess>,
    pub(crate) component: Component,
    pub(crate) db_scale: bool,
}

#[allow(dead_code)]
impl Process {
    pub(crate) fn new_freq_domain() -> Self {
        Self {
            fft: Some(Default::default()),
            db_scale: true,
            ..Default::default()
        }
    }

    /* pub fn proc_by_ref(&self, data: &[Complex64]) -> Vec<f64> {
        let mut data = data.to_owned();
        if let Some(mut fft) = self.fft.as_ref().cloned() {
            let (f, b) = fft.get_fft(data.len());
            debug_assert_eq!(b.len(), f.get_inplace_scratch_len());
            f.process_with_scratch(&mut data, b);
            let split_pos = (data.len() + 1) / 2; //for odd situations, need to shift (len+1)/2..len, for evens, len/2..len
            let (pos_freq, neg_freq) = data.split_at_mut(split_pos);
            data = neg_freq.iter().chain(pos_freq.iter()).copied().collect();
        }

        if self.db_scale {
            self.component
                .extract(data.into_iter())
                .map({ |x: f64| x.log10() * 20. } as fn(_) -> _)
                .collect()
        } else {
            self.component.extract(data.into_iter()).collect()
        }
    } */

    pub fn proc(&mut self, data: &[Complex64]) -> Vec<f64> {
        //puffin::profile_function!();
        let Process {
            fft,
            component,
            db_scale,
        } = self;
        let mut data = data.to_owned();
        if let Some((f, b)) = fft.as_mut().map(|x| x.get_fft(data.len())) {
            debug_assert_eq!(b.len(), f.get_inplace_scratch_len());
            f.process_with_scratch(&mut data, b);
            let split_pos = (data.len() + 1) / 2; //for odd situations, need to shift (len+1)/2..len, for evens, len/2..len
            let (pos_freq, neg_freq) = data.split_at_mut(split_pos);
            data = neg_freq.iter().chain(pos_freq.iter()).copied().collect();
        }

        if *db_scale {
            component
                .extract(data.into_iter())
                .map({ |x: f64| x.log10() * 20. } as fn(_) -> _)
                .collect()
        } else {
            component.extract(data.into_iter()).collect()
        }
    }

    pub fn proc_f32_by_ref(&self, data: &[Complex64]) -> Vec<f32> {
        let mut data = data.to_owned();
        if let Some(mut fft) = self.fft.as_ref().cloned() {
            let (f, b) = fft.get_fft(data.len());
            debug_assert_eq!(b.len(), f.get_inplace_scratch_len());
            f.process_with_scratch(&mut data, b);
            let split_pos = (data.len() + 1) / 2; //for odd situations, need to shift (len+1)/2..len, for evens, len/2..len
            let (pos_freq, neg_freq) = data.split_at_mut(split_pos);
            data = neg_freq.iter().chain(pos_freq.iter()).copied().collect();
        }

        if self.db_scale {
            self.component
                .extract(data.into_iter())
                .map({ |x: f64| (x as f32).log10() * 20. } as fn(_) -> _)
                .collect()
        } else {
            self.component.extract_f32(data.into_iter()).collect()
        }
    }

    pub fn proc_f32(&mut self, data: &[Complex64]) -> Vec<f32> {
        //puffin::profile_function!();
        let Process {
            fft,
            component,
            db_scale,
        } = self;
        let mut data = data.to_owned();
        if let Some((f, b)) = fft.as_mut().map(|x| x.get_fft(data.len())) {
            debug_assert_eq!(b.len(), f.get_inplace_scratch_len());
            f.process_with_scratch(&mut data, b);
            let split_pos = (data.len() + 1) / 2; //for odd situations, need to shift (len+1)/2..len, for evens, len/2..len
            let (pos_freq, neg_freq) = data.split_at_mut(split_pos);
            data = neg_freq.iter().chain(pos_freq.iter()).copied().collect();
        }

        if *db_scale {
            component
                .extract(data.into_iter())
                .map({ |x: f64| ((x as f32).log10() * 20.) as _ } as fn(_) -> _)
                .collect()
        } else {
            component.extract_f32(data.into_iter()).collect()
        }
    }

    pub(crate) fn controller(&mut self, ui: &mut egui::Ui) {
        crate::util::toggle_option(ui, &mut self.fft, "FFT");
        ui.separator();
        self.component.show(ui);
        ui.separator();
        ui.toggle_value(&mut self.db_scale, "dB scale");
    }
}

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    enum_iterator::Sequence,
)]
pub enum Component {
    Real,
    Imag,
    #[default]
    Abs,
    Arg,
}

impl Component {
    pub fn desc(&self) -> &str {
        match self {
            Component::Real => "Real",
            Component::Imag => "Imag",
            Component::Abs => "Abs",
            Component::Arg => "Arg",
        }
    }
    pub fn extract<B: Iterator<Item = Complex64>>(&self, i: B) -> Map<B, fn(Complex64) -> f64> {
        match self {
            Component::Real => i.map({ |x| x.re } as fn(Complex64) -> f64),
            Component::Imag => i.map({ |x| x.im } as fn(Complex64) -> f64),
            Component::Abs => i.map({ |x| x.abs() } as fn(Complex64) -> f64),
            Component::Arg => i.map({ |x| x.arg() } as fn(Complex64) -> f64),
        }
    }
    pub fn extract_f32<B: Iterator<Item = Complex64>>(&self, i: B) -> Map<B, fn(Complex64) -> f32> {
        match self {
            Component::Real => i.map({ |x| x.re as _ } as fn(Complex64) -> f32),
            Component::Imag => i.map({ |x| x.im as _ } as fn(Complex64) -> f32),
            Component::Abs => i.map({ |x| x.abs() as _ } as fn(Complex64) -> f32),
            Component::Arg => i.map({ |x| x.arg() as _ } as fn(Complex64) -> f32),
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        enum_iterator::all::<Component>().for_each(|s| {
            if ui.selectable_label(self == &s, s.desc()).clicked() {
                *self = s;
            }
        })
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct FftProcess {
    #[serde(skip)]
    s: Option<(Fft, Vec<Complex64>)>,
}

impl FftProcess {
    pub(crate) fn get_fft(&mut self, len: usize) -> &mut (Fft, Vec<Complex64>) {
        if self.target_len().is_some_and(|x| x != len) {
            crate::notify::TOASTS
                .lock()
                .warning("Unmatched FftProcess length, reinitializing");
            self.s = None;
        }
        self.s.get_or_insert_with(|| {
            let f = FftPlanner::new().plan_fft_forward(len);
            let buf = vec![Complex64::zero(); f.get_inplace_scratch_len()];
            (f, buf)
        })
    }

    pub(crate) fn target_len(&self) -> Option<usize> {
        self.s.as_ref().map(|x| x.1.len())
    }
}

impl Clone for FftProcess {
    fn clone(&self) -> Self {
        Self { s: None }
    }
}

impl Debug for FftProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FftProcess")
            .field("s", &"dyn type")
            .finish()
    }
}
