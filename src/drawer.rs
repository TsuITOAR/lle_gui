use lle::num_traits::{Float, FromPrimitive};
use std::{fmt::Debug, ops::RangeInclusive};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlotRange<T> {
    bounds: (Bound<T>, Bound<T>),
    scale_radix: Option<u32>,
}

impl<T: Debug + Float + PartialOrd + FromPrimitive + Copy> PlotRange<T> {
    pub fn new(bound: Bound<T>, scale_radix: impl Into<Option<u32>>) -> Self {
        Self {
            bounds: (bound.clone(), bound),
            scale_radix: scale_radix.into(),
        }
    }
    #[allow(unused)]
    pub fn new2(bounds: (Bound<T>, Bound<T>), scale_radix: impl Into<Option<u32>>) -> Self {
        Self {
            bounds,
            scale_radix: scale_radix.into(),
        }
    }

    #[allow(unused)]
    pub fn set_last(&mut self, last: RangeInclusive<T>) -> &mut Self {
        assert!(!last.is_empty());
        self.bounds.0.v = Some(*last.start());
        self.bounds.1.v = Some(*last.end());
        self
    }

    pub fn update(&mut self, new: RangeInclusive<T>) -> RangeInclusive<T> {
        assert!(!new.is_empty());
        let mag = self.scale_radix.map(|radix| {
            let radix: T = T::from_u32(radix).unwrap();
            let dis = *new.end() - *new.start();
            let order = dis.log(radix).ceil() - T::one();
            radix.powf(order)
        });
        self.bounds.0.update_as_lower(*new.start(), mag)
            ..=self.bounds.1.update_as_upper(*new.end(), mag)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum PlotStrategy {
    Static,
    InstantFit,
    LazyFit { max_lazy: u32, lazy: u32 },
    GrowOnly,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Bound<T> {
    v: Option<T>,
    strategy: PlotStrategy,
}

impl<T: Debug + Float + PartialOrd + FromPrimitive + Copy> Bound<T> {
    pub fn new(strategy: PlotStrategy) -> Self {
        Self { v: None, strategy }
    }
    pub fn update_as_lower(&mut self, new: T, mag: Option<T>) -> T {
        fn min<T: PartialOrd + Copy>(a: &T, b: &T) -> T {
            if a.le(b) {
                *a
            } else {
                *b
            }
        }
        let mut new = new;
        if let Some(mag) = mag {
            let sta = new;
            let mag_sta = ((sta / mag).ceil() - T::one()) * mag;
            new = mag_sta;
        }
        *match self.v {
            Some(ref mut last_used) => {
                match self.strategy {
                    PlotStrategy::Static => (),
                    PlotStrategy::InstantFit => {
                        *last_used = new;
                    }
                    PlotStrategy::LazyFit {
                        max_lazy,
                        ref mut lazy,
                    } => {
                        if (*last_used).lt(&new) && *lazy < max_lazy {
                            *lazy += 1;
                        } else {
                            *lazy = 0;
                            *last_used = new;
                        }
                    }
                    PlotStrategy::GrowOnly => {
                        *last_used = min(&new, last_used);
                    }
                };
                last_used
            }
            None => self.v.insert(new),
        }
    }
    pub fn update_as_upper(&mut self, new: T, mag: Option<T>) -> T {
        fn max<T: PartialOrd + Copy>(a: &T, b: &T) -> T {
            if a.gt(b) {
                *a
            } else {
                *b
            }
        }
        let mut new = new;
        if let Some(mag) = mag {
            let end = new;
            let mag_end = ((end / mag).floor() + T::one()) * mag;
            new = mag_end;
        }
        *match self.v {
            Some(ref mut last_used) => {
                match self.strategy {
                    PlotStrategy::Static => (),
                    PlotStrategy::InstantFit => {
                        *last_used = new;
                    }
                    PlotStrategy::LazyFit {
                        max_lazy,
                        ref mut lazy,
                    } => {
                        if (*last_used).gt(&new) && *lazy < max_lazy {
                            *lazy += 1;
                        } else {
                            *lazy = 0;
                            *last_used = new;
                        }
                    }
                    PlotStrategy::GrowOnly => {
                        *last_used = max(&new, last_used);
                    }
                };
                last_used
            }
            None => self.v.insert(new),
        }
    }
}
