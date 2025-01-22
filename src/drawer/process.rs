use std::iter::Map;

use lle::num_complex::ComplexFloat;

use super::*;

pub trait FftSource:
    lle::FftSource<f64>
    + AsMut<[Complex64]>
    + AsRef<[Complex64]>
    + Sync
    + Clone
    + 'static
    + Debug
    + Send
    + Sync
    + From<Vec<Complex64>>
{
    fn default_with_len(len: usize) -> Self {
        let v = vec![Complex64::default(); len];
        v.into()
    }
}

impl<
        T: lle::FftSource<f64>
            + AsMut<[Complex64]>
            + AsRef<[Complex64]>
            + Sync
            + Clone
            + 'static
            + Debug
            + Send
            + Sync
            + From<Vec<Complex64>>,
    > FftSource for T
{
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Process<S: FftSource> {
    pub(crate) fft: Option<FftProcess<S>>,
    pub(crate) component: Component,
    pub(crate) db_scale: bool,
}

impl<S: FftSource> Default for Process<S> {
    fn default() -> Self {
        Self {
            fft: None,
            component: Default::default(),
            db_scale: false,
        }
    }
}

#[allow(dead_code)]
impl<S: FftSource> Process<S> {
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

    pub fn proc(&mut self, data: &S) -> Vec<f64> {
        //puffin::profile_function!();
        let Process {
            fft,
            component,
            db_scale,
        } = self;
        let mut data = data.to_owned();
        let data = if let Some((f, _)) = fft.as_mut().map(|x| x.get_fft(data.fft_len())) {
            data.fft_process_forward(f);
            let data_slice = data.as_mut();
            let split_pos = (data_slice.len() + 1) / 2; //for odd situations, need to shift (len+1)/2..len, for evens, len/2..len
            let (pos_freq, neg_freq) = data_slice.split_at_mut(split_pos);
            neg_freq.iter().chain(pos_freq.iter()).copied().collect()
        } else {
            data.as_ref().to_owned()
        };

        if *db_scale {
            component
                .extract(data.into_iter())
                .map({ |x: f64| x.log10() * 20. } as fn(_) -> _)
                .collect()
        } else {
            component.extract(data.into_iter()).collect()
        }
    }

    pub fn proc_f32_by_ref(&self, data: &S) -> Vec<f32> {
        let mut data = data.to_owned();
        let data =
            if let Some((f, _)) = self.fft.clone().as_mut().map(|x| x.get_fft(data.fft_len())) {
                data.fft_process_forward(f);
                let data_slice = data.as_mut();
                let split_pos = (data_slice.len() + 1) / 2; //for odd situations, need to shift (len+1)/2..len, for evens, len/2..len
                let (pos_freq, neg_freq) = data_slice.split_at_mut(split_pos);
                neg_freq.iter().chain(pos_freq.iter()).copied().collect()
            } else {
                data.as_ref().to_owned()
            };

        if self.db_scale {
            self.component
                .extract(data.into_iter())
                .map({ |x: f64| (x as f32).log10() * 20. } as fn(_) -> _)
                .collect()
        } else {
            self.component.extract_f32(data.into_iter()).collect()
        }
    }

    pub fn proc_f32(&mut self, data: &S) -> Vec<f32> {
        //puffin::profile_function!();
        let Process {
            fft,
            component,
            db_scale,
        } = self;
        let mut data = data.to_owned();
        let data = if let Some((f, _)) = fft.as_mut().map(|x| x.get_fft(data.fft_len())) {
            data.fft_process_forward(f);
            let data_slice = data.as_mut();
            let split_pos = (data_slice.len() + 1) / 2; //for odd situations, need to shift (len+1)/2..len, for evens, len/2..len
            let (pos_freq, neg_freq) = data_slice.split_at_mut(split_pos);
            neg_freq.iter().chain(pos_freq.iter()).copied().collect()
        } else {
            data.as_ref().to_owned()
        };

        if *db_scale {
            component
                .extract(data.into_iter())
                .map({ |x: f64| ((x as f32).log10() * 20.) as _ } as fn(_) -> _)
                .collect()
        } else {
            component.extract_f32(data.into_iter()).collect()
        }
    }
}

impl<S: FftSource> ui_traits::ControllerUI for Process<S> {
    fn show_controller(&mut self, ui: &mut egui::Ui) {
        crate::util::show_option(ui, &mut self.fft, "FFT");
        ui.separator();
        self.component.show_controller(ui);
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
    Eq,
    PartialOrd,
    Ord,
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

impl crate::util::DisplayStr for Component {
    fn desc(&self) -> &str {
        match self {
            Component::Real => "Real",
            Component::Imag => "Imag",
            Component::Abs => "Abs",
            Component::Arg => "Arg",
        }
    }
}

impl Component {
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
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct FftProcess<S: lle::FftSource<f64>> {
    #[serde(skip)]
    s: Option<(S::FftProcessor, usize)>,
}

impl<S: lle::FftSource<f64>> Default for FftProcess<S> {
    fn default() -> Self {
        Self { s: None }
    }
}

impl<S: lle::FftSource<f64>> FftProcess<S> {
    pub(crate) fn get_fft(&mut self, len: usize) -> &mut (S::FftProcessor, usize) {
        if self.target_len().is_some_and(|x| x != len) {
            crate::notify::TOASTS
                .lock()
                .warning("Unmatched FftProcess length, reinitializing");
            self.s = None;
        }
        self.s.get_or_insert_with(|| {
            debug_assert!(len % 2 == 0);
            (S::default_fft(len), len)
        })
    }

    pub(crate) fn target_len(&self) -> Option<usize> {
        self.s.as_ref().map(|x| x.1)
    }
}

impl<S: lle::FftSource<f64>> Clone for FftProcess<S> {
    fn clone(&self) -> Self {
        Self { s: None }
    }
}

impl<S: lle::FftSource<f64>> Debug for FftProcess<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FftProcess")
            .field("s", &"dyn type")
            .finish()
    }
}
