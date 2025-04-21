use std::f64::consts::TAU;

use lle::{num_complex::Complex64, Freq, LinearOp, StaticLinearOp, Step};

use crate::controller::cprt2::CoupleStrength;

impl StaticLinearOp<f64> for CprtDispersionFrac {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CprtDispersionFrac {
    #[serde(alias = "center")]
    pub(crate) center_pos: f64,
    pub(crate) period: f64,
    #[serde(default)]
    pub(crate) couple_strength: CoupleStrength,
    pub(crate) frac_d1_2pi: f64,
}

impl CprtDispersionFrac {
    fn m_period(&self) -> f64 {
        // /2 for half 2pi, *2 for double modes
        self.period
    }

    pub(crate) fn m_original(&self, freq: Freq) -> i32 {
        let f = freq as f64 - self.center_pos * 2. + self.period / 2.;
        f.div_euclid(self.m_period()) as i32
    }

    pub fn fraction_at(&self, mode: i32) -> ((f64, f64), (f64, f64)) {
        let m = mode as f64;

        let phi_m = TAU * (m - self.center_pos) / self.period;
        let alpha = (self.couple_strength.get_coupling(m).cos() * phi_m.cos()).acos();
        let cp_angle = f64::atan2(
            (alpha + phi_m).sin().abs().sqrt(),
            (alpha - phi_m).sin().abs().sqrt(),
        );
        (
            (cp_angle.cos(), cp_angle.sin()),
            (-cp_angle.sin(), cp_angle.cos()),
        )
    }
}

impl LinearOp<f64> for CprtDispersionFrac {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        let m = self.m_original(freq);
        let branch = (freq - m).rem_euclid(2);
        debug_assert!(branch == 0 || branch == 1);
        let f = |f: Freq, m: Freq| {
            let f = (f - m).div_euclid(2) as f64;
            let cos1 = (((f - self.center_pos) / self.period) * TAU).cos().abs();
            let couple_strength = self.couple_strength.get_coupling(f);
            //dbg!(f, couple_strength);
            let cos2 = couple_strength.cos();

            ((cos1 * cos2).acos()) * self.frac_d1_2pi
        };

        if branch == 1 {
            -Complex64::i() * (f(freq, m) - f(1, 0))
        } else {
            -Complex64::i() * (-f(freq, m) - f(1, 0))
        }
    }
    fn skip(&self) -> bool {
        false
    }
}
