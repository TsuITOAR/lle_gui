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

    // freq counting for two modes, not pairs
    pub(crate) fn m_original(&self, freq: Freq) -> i32 {
        let f = freq as f64 - self.center_pos * 2. + self.period / 2.;
        f.div_euclid(self.m_period()) as i32
    }
    /// freq0 the real number
    pub fn singularity_point(&self, freq: lle::Freq) -> bool {
        let freq = freq as f64 - self.center_pos * 2.;
        let diff = (freq + self.period / 2.).rem_euclid(self.period);
        (0. ..1.).contains(&diff)
    }
    /// freq the pair number
    pub fn fraction_at(
        &self,
        freq: i32,
        m: i32,
        time: f64,
    ) -> ((Complex64, Complex64), (Complex64, Complex64)) {
        let freq = freq as f64;
        let d1 = self.frac_d1_2pi * TAU;
        let phi_m = TAU * (freq - self.center_pos) / self.period;
        let alpha = (self.couple_strength.get_coupling(freq).cos() * phi_m.cos()).acos();
        let cp_angle = f64::atan2(
            (alpha + phi_m).sin().abs().sqrt(),
            (alpha - phi_m).sin().abs().sqrt(),
        );
        let spatial_move_term = spatial_basis_move(m, d1, time);
        (
            (
                cp_angle.cos() * spatial_move_term,
                cp_angle.sin() * spatial_move_term.conj(),
            ),
            (
                -cp_angle.sin() * spatial_move_term,
                cp_angle.cos() * spatial_move_term.conj(),
            ),
        )
    }
}

pub(crate) fn spatial_basis_move(m: i32, d1: f64, time: f64) -> Complex64 {
    (Complex64::I * m as f64 / 2. * d1 * time).exp()
    // Complex64::i()
}

impl LinearOp<f64> for CprtDispersionFrac {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        let m = self.m_original(freq);
        let branch = (freq + m).rem_euclid(2);
        let f_eff = (freq + m).div_euclid(2);
        debug_assert!(branch == 0 || branch == 1);
        let f = |f: Freq| {
            let cos1 = (((f as f64 - self.center_pos) / self.period) * TAU)
                .cos()
                .abs();
            let couple_strength = self.couple_strength.get_coupling(f as f64);
            //dbg!(f, couple_strength);
            let cos2 = couple_strength.cos();

            ((cos1 * cos2).acos()) * self.frac_d1_2pi
        };

        if branch == 1 {
            -Complex64::i() * (f(f_eff) - f(0))
        } else {
            -Complex64::i() * (-f(f_eff) - f(0))
        }
    }
    fn skip(&self) -> bool {
        false
    }
}
