use lle::{Freq, LinearOp, StaticLinearOp, Step, num_complex::Complex64};

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
    // freq counting for two modes, not pairs
    // original should change after even modes
    pub(crate) fn m_original(&self, freq: Freq) -> i32 {
        let f = freq as f64 + self.period / 2. - self.center_pos * 2.;
        f.div_euclid(self.period) as i32
    }
    /// freq0 the real number
    pub fn singularity_point(&self, freq: lle::Freq) -> bool {
        // return false;
        let freq = freq as f64 - self.center_pos * 2.;
        let diff = (freq + self.period / 2.).rem_euclid(self.period) as i32;
        diff == 0
        // if freq > 0 {
        //     self.m_original(freq) != self.m_original(freq + 1)
        // } else {
        //     self.m_original(freq) != self.m_original(freq - 1)
        // }
    }

    pub fn branch_at_upper(&self, freq: Freq) -> bool {
        // let m = self.m_original(freq);
        let branch = (freq).rem_euclid(2);
        // if self.singularity_point(freq) {
        //     m > 0
        // } else {
        //     branch == 1
        // }
        branch == 1
    }

    pub(crate) fn phi_m(&self, freq: f64) -> f64 {
        use std::f64::consts::TAU;
        TAU * (freq - self.center_pos) / self.period
    }

    pub(crate) fn cp_angle(&self, freq: i32, _m: i32) -> f64 {
        // use std::f64::consts::*;
        let freq = freq as f64;
        let phi_m = self.phi_m(freq);
        let alpha = (self.couple_strength.get_coupling(freq).cos() * phi_m.cos()).acos();
        f64::atan2(
            (alpha + phi_m).sin().abs().sqrt(),
            (alpha - phi_m).sin().abs().sqrt(),
        )
    }

    /// freq the pair number
    pub fn fraction_at(
        &self,
        freq: i32,
        m: i32,
        time: f64,
    ) -> ((Complex64, Complex64), (Complex64, Complex64)) {
        // todo: only works when no singularity!!!
        use std::f64::consts::*;
        let d1 = self.frac_d1_2pi * TAU;
        let cp_angle = self.cp_angle(freq, m);
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
        // (((1.).into(), (0.).into()), ((0.).into(), (1.).into()))
    }
}

pub(crate) fn spatial_basis_move(m: i32, d1: f64, time: f64) -> Complex64 {
    (Complex64::I * m as f64 / 2. * d1 * time).exp()
    // Complex64::i()
}

impl LinearOp<f64> for CprtDispersionFrac {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        //todo: test the freq brach selection is consistent with walk off freq iter impl

        use std::f64::consts::*;
        let m = self.m_original(freq);
        let f_eff = (freq + m).div_euclid(2);
        // return -Complex64::i()*m as f64;
        let branch_upper = self.branch_at_upper(freq);
        let f = |f: Freq| {
            let cos1 = (((f as f64 - self.center_pos) / self.period) * TAU)
                .cos()
                .abs();
            let couple_strength = self.couple_strength.get_coupling(f as f64);
            //dbg!(f, couple_strength);
            let cos2 = couple_strength.cos();

            ((cos1 * cos2).acos()) * self.frac_d1_2pi
        };

        // -Complex64::i() * (self.cp_angle((freq + m) / 2, m) / PI + m as f64 * 3.) +
        if branch_upper {
            -Complex64::i() * (f(f_eff) - f(0))
        } else {
            -Complex64::i() * (-f(f_eff) - f(0))
        }
    }
    fn skip(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_singularity() {
        let cp = super::CprtDispersionFrac {
            center_pos: 1.0,
            period: 11.0,
            couple_strength: super::CoupleStrength::default(),
            frac_d1_2pi: 1.0,
        };
        let mut last_m = None;
        for f in -100..100 {
            let m = cp.m_original(f);
            if cp.singularity_point(f) {
                println!("singularity at f = {f}, m = {m}, last_m = {last_m:?}");
            } else if cp.singularity_point(f - 1) {
                assert!(last_m.map(|last_m| m == last_m).unwrap_or(true));
                println!("first freq after singularity at f = {f}, m = {m}, last_m = {last_m:?}");
            }
            last_m = Some(m);
        }
    }
    /* #[test]
    fn test_branch() {
        let cp = super::CprtDispersionFrac {
            center_pos: 1.0,
            period: 10.0,
            couple_strength: super::CoupleStrength::default(),
            frac_d1_2pi: 1.0,
        };
        let mut last_branch = None;
        for f in -100..100 {
            let branch = cp.branch_at_upper(f);
            let m = cp.m_original(f);
            if cp.singularity_point(f) {
                println!(
                    "singularity at f = {f}, m = {m}, brach = {branch}, last_brach = {last_branch:?}"
                );
                if m > 0 {
                    assert!(branch);
                } else {
                    assert!(!branch);
                }
            } else if cp.singularity_point(f - 1) {
                println!(
                    "after singularity at f = {f}, m = {m}, brach = {branch}, last_brach = {last_branch:?}"
                );
                assert!(!branch);
            } else if cp.singularity_point(f + 1) {
                println!(
                    "before singularity at f = {f}, m = {m}, brach = {branch}, last_brach = {last_branch:?}"
                );
                assert!(branch);
                if let Some(last_branch) = last_branch {
                    assert!(last_branch != branch);
                }
            }
            last_branch = Some(branch);
        }
    } */

    /* #[test]
    fn test_disper_m_consistent() {
        use crate::controller::gencprt::state::{Mode, State};
        use crate::controller::gencprt::TEST_DATA;
        let cp = super::CprtDispersionFrac {
            center_pos: 1.0,
            period: 11.0,
            couple_strength: super::CoupleStrength::default(),
            frac_d1_2pi: 1.0,
        };
        let state = State {
            data: TEST_DATA.to_vec(),
            cp: cp.clone(),
            time: 0.0,
        };
        let freq_iter_p = state.coupling_iter_positive();
        let freq_iter_n = state.coupling_iter_negative();

        let len = state.data.len();

        let (state_a, state_b) = state.data.split_at(len / 2);

        for f in freq_iter_p {
            let index = lle::index_at(len, f.meta().freq);
            match f {
                Mode::Single { amp, .. } => {
                    assert_eq!(state_a[index], amp, "index: {index}, f: {f:?}");
                }
                Mode::Pair { amp1, amp2, .. } => {
                    assert_eq!(state_a[index], amp1, "index: {index}, f: {f:?}");
                    assert_eq!(state_b[index], amp2, "index: {index}, f: {f:?}");
                }
            }
        }
    } */
}
