use lle::num_traits::{Float, FromPrimitive};
use std::{fmt::Debug, ops::RangeInclusive};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlotRange<T> {
    pub(crate) dis: T,
    pub(crate) center: T,
    pub(crate) lazy_count: (u32, u32),
    pub(crate) adapt: (u32, u32, u32, u32),
    pub(crate) scale_radix: u32,
}

impl<T: Debug + Float + PartialOrd + FromPrimitive + Copy> PlotRange<T> {
    pub fn new(
        scale_radix: u32,
        max_lazy: u32,
        min_refresh_adapt: u32,
        max_refresh_adapt: u32,
        max_adapt_factor: u32,
    ) -> Self {
        Self {
            dis: T::one(),
            center: T::zero(),
            scale_radix,
            lazy_count: (0, max_lazy),
            adapt: (1, min_refresh_adapt, max_refresh_adapt, max_adapt_factor),
        }
    }
    fn current_bound(&self) -> RangeInclusive<T> {
        let div = T::from_f64(2f64).unwrap();
        (self.center - self.dis / div)..=(self.center + self.dis / div)
    }
    pub fn update(&mut self, new: RangeInclusive<T>) -> RangeInclusive<T> {
        assert!(!new.is_empty());
        let cbound = self.current_bound();
        if cbound.contains(new.start())
            && cbound.contains(new.end())
            && self.lazy_count.0 < self.lazy_count.1
        {
            self.lazy_count.0 += 1;
            return cbound;
        }
        if self.lazy_count.0 < self.adapt.1 {
            self.adapt.0 = (self.adapt.0 + 1).min(self.adapt.3)
        } else if self.lazy_count.0 > self.adapt.2 {
            self.adapt.0 = self.adapt.0.max(6) - 5;
        }
        self.lazy_count.0 = 0;
        let dis = *new.end() - *new.start();
        let mag = {
            let radix: T = T::from_u32(self.scale_radix).unwrap();
            let order = dis.log(radix).ceil() - T::one();
            radix.powf(order)
        } * T::from_u32(self.adapt.0).unwrap();
        let div = T::from_f64(2f64).unwrap();
        let center = (*new.end() + *new.start()) / div;
        let dis = (((*new.end() - *new.start()) / mag).floor() + T::one()) * mag;
        self.center = center;
        self.dis = dis;
        (center - dis / div)..=(center + dis / div)
    }
}
