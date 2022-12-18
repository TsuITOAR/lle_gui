use std::{mem, sync::Arc, thread::JoinHandle};

use anyhow::{anyhow, Result};
use jkplot::{RawAnimator, RawMapVisualizer};
use lle::{
    num_complex::Complex64,
    rustfft::{Fft, FftPlanner},
};
use log::warn;
use plotters::prelude::*;
use plotters_bitmap::BitMapBackend;
pub struct DrawData {
    pub(crate) data: Vec<Vec<Complex64>>,
    pub(crate) plot_real: RawAnimator,
    pub(crate) plot_freq: RawAnimator,
    pub(crate) fft: Arc<dyn Fft<f64>>,
    //pub(crate) map: SpawnMapVisual,
    pub(crate) size: (usize, usize),
    pub(crate) buffer_up: Vec<u8>,
    pub(crate) buffer_down: Vec<u8>,
}

struct StandBy {
    bitmap_buffer: Vec<u8>,
    data: Vec<Vec<f64>>,
    map: RawMapVisualizer,
    size: (usize, usize),
    index: usize,
}

#[allow(unused)]
impl StandBy {
    fn new(size: (usize, usize)) -> Self {
        Self {
            bitmap_buffer: vec![0; size.0 * size.1 * 4], //4 for rgba
            data: Vec::new(),
            map: RawMapVisualizer::default(),
            size,
            index: 0,
        }
    }
    fn draw(&mut self) -> Result<()> {
        self.map.draw_on(
            &self.data,
            &BitMapBackend::<plotters_bitmap::bitmap_pixel::BGRXPixel>::with_buffer_and_format(
                &mut self.bitmap_buffer,
                (self.size.0 as u32, self.size.1 as u32),
            )?
            .into_drawing_area(),
        )?;
        Ok(())
    }
    fn spawn(mut self) -> JoinHandle<Self> {
        std::thread::spawn(move || {
            self.draw().expect("failed drawing color map");
            self
        })
    }
}

enum SpawnMapVisual {
    StandBy(StandBy),
    Handler(JoinHandle<StandBy>),
    ///this should never appear other than its own method for temporary take the data ownership
    Temp,
}

#[allow(unused)]
impl SpawnMapVisual {
    fn new(size: (usize, usize)) -> Self {
        SpawnMapVisual::StandBy(StandBy::new(size))
    }
    // TODO: find a solution to synchronize the buffer size of DrawData and SpawnMapVisual
    fn try_update(
        &mut self,
        new_data: &mut Vec<Vec<Complex64>>,
        buffer_dis: &mut [u8],
    ) -> Result<bool> {
        const MAX_RECORD_LEN: usize = 500;
        let mut ret = true;
        if let SpawnMapVisual::Handler(h) = std::mem::replace(self, SpawnMapVisual::Temp) {
            if h.is_finished() {
                *self = SpawnMapVisual::StandBy(
                    h.join().map_err(|_| anyhow!("color map thread panicked"))?,
                );
            } else {
                ret = false;
                *self = SpawnMapVisual::Handler(h);
            }
        }
        if let SpawnMapVisual::StandBy(mut s) = std::mem::replace(self, SpawnMapVisual::Temp) {
            s.data.reserve(new_data.len());
            mem::take(new_data).into_iter().for_each(|x| {
                let temp = x.into_iter().map(|x| x.re).collect::<Vec<_>>();
                s.map.update_range(&temp);
                if s.data.len() < MAX_RECORD_LEN {
                    s.data.push(temp);
                } else {
                    s.data[s.index] = temp;
                    s.index = (s.index + 1) % MAX_RECORD_LEN;
                }
            });
            buffer_dis.clone_from_slice(&s.bitmap_buffer);
            *self = SpawnMapVisual::Handler(s.spawn());
        }
        Ok(ret)
    }
}

impl DrawData {
    fn split_area(size: usize) -> (usize, usize) {
        (size / 2, size - size / 2)
    }
    pub fn new(data_len: usize, window_size: (usize, usize)) -> Self {
        DrawData {
            data: Vec::default(),
            plot_real: RawAnimator::default(),
            plot_freq: {
                let mut a = RawAnimator::default();
                a.set_y_desc("dB");
                a.set_x_label_formatter(move |x| format!("{}", (x - (data_len / 2) as f64)));
                a
            },
            fft: FftPlanner::new().plan_fft_forward(data_len),
            //map: SpawnMapVisual::new((window_size.0, window_size.1)),
            size: window_size,
            buffer_up: vec![0; window_size.0 * window_size.1 * 4],
            buffer_down: vec![0; window_size.0 * window_size.1 * 4],
        }
    }
    pub fn resize(&mut self, size: (usize, usize)) {
        self.buffer_up.resize(size.0 * size.1 * 4, 0);
        self.buffer_down.resize(size.0 * size.1 * 4, 0);
    }
    pub fn update(&mut self) -> Result<()> {
        //get or create window
        let size = self.size;
        //draw chart
        let (upper_buffer, lower_buffer) = (&mut self.buffer_up, &mut self.buffer_down);
        let upper =
            BitMapBackend::<plotters_bitmap::bitmap_pixel::BGRXPixel>::with_buffer_and_format(
                upper_buffer,
                (size.0 as u32, size.1 as u32),
            )?
            .into_drawing_area();
        let lower =
            BitMapBackend::<plotters_bitmap::bitmap_pixel::BGRXPixel>::with_buffer_and_format(
                lower_buffer,
                (size.0 as u32, size.1 as u32),
            )?
            .into_drawing_area();
        if let Some(d) = self.data.last() {
            self.plot_real
                .new_frame_on(d.iter().enumerate().map(|(x, y)| (x as f64, y.re)), &upper)
                .unwrap();
            let mut freq = d.to_owned();
            self.fft.process(&mut freq);
            let (first, second) = freq.split_at(freq.len() / 2);

            self.plot_freq
                .new_frame_on(
                    second
                        .iter()
                        .chain(first.iter())
                        .enumerate()
                        .map(|(x, y)| (x as f64, 10. * (y.norm().log10()))),
                    &lower,
                )
                .unwrap();
            //self.map.try_update(&mut self.data, &mut self.buffer_down)?;
        } else {
            warn!("trying drawing empty data");
        }

        Ok(())
    }
    pub fn fetch(&mut self) -> Result<((usize, usize), &[u8], &[u8])> {
        Ok((self.size, &self.buffer_up, &self.buffer_down))
    }
    pub fn push(&mut self, new_data: Vec<Complex64>) {
        self.data.push(new_data);
    }
}
