use std::{fmt::Debug, ops::RangeInclusive};
use lle::num_traits::{FromPrimitive, Float};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlotRange<T> {
    last_used: Option<RangeInclusive<T>>,
    strategy: PlotStrategy,
    scale_radix: Option<u32>,
}

impl<T: Debug + Float + PartialOrd + FromPrimitive + Copy> PlotRange<T> {
    pub fn new(strategy: PlotStrategy, scale_radix: impl Into<Option<u32>>) -> Self {
        Self {
            last_used: None,
            strategy,
            scale_radix: scale_radix.into(),
        }
    }

    pub fn set_last(&mut self, last: RangeInclusive<T>) -> &mut Self {
        assert!(!last.is_empty());
        self.last_used = Some(last);
        self
    }

    pub fn update(&mut self, new: RangeInclusive<T>) -> RangeInclusive<T> {
        assert!(!new.is_empty());
        fn max<T: PartialOrd + Copy>(a: &T, b: &T) -> T {
            if a.gt(b) {
                *a
            } else {
                *b
            }
        }
        fn min<T: PartialOrd + Copy>(a: &T, b: &T) -> T {
            if a.le(b) {
                *a
            } else {
                *b
            }
        }
        let last_used = match self.last_used {
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
                        if last_used.start().le(new.start())
                            && last_used.end().ge(new.end())
                            && *lazy < max_lazy
                        {
                            *lazy += 1;
                        } else {
                            *lazy = 0;
                            *last_used = *new.start()..=*new.end();
                        }
                    }
                    PlotStrategy::GrowOnly => {
                        *last_used =
                            min(new.start(), last_used.start())..=max(new.end(), last_used.end());
                    }
                };
                last_used
            }
            None => self.last_used.insert(new),
        };
        if let Some(radix) = self.scale_radix {
            let radix: T = T::from_u32(radix).unwrap();
            let sta = *last_used.start();
            let end = *last_used.end();
            let dis = end - sta;
            let order = dis.log(radix).ceil() - T::one();
            let mag = radix.powf(order);
            let mag_sta = (sta / mag).floor() * mag;
            let mag_end = (end / mag).ceil() * mag;
            *last_used = mag_sta..=mag_end;
        }
        last_used.clone()
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum PlotStrategy {
    Static,
    InstantFit,
    LazyFit { max_lazy: u8, lazy: u8 },
    GrowOnly,
}
