// my own libs
use dsp_lab::core::delay::{DelayLine, MixMethod, InterpMethod};
use dsp_lab::core::lin_filter::{DcBlock, SvfLowPass, LowPass1P};
use dsp_lab::core::chaos::{NoiseWhite, SnhRandom};
use dsp_lab::core::DenormalDither;
use dsp_lab::core::osc::{ParOsc, AsymTriOsc};
use dsp_lab::traits::{Source, Process};
use dsp_lab::emulation::Hysteresis;
use dsp_lab::utils::math::x_fade;


pub struct Dropouts {
    lfo_1: SnhRandom,
    lfo_2: SnhRandom,
}

impl Dropouts {
    pub fn new() -> Self {
        let mut ret = Self {
            lfo_1: SnhRandom::new(44100.0, 10),
            lfo_2: SnhRandom::new(44100.0, 11),
        };
        ret.lfo_1.set_freq(7.04739733304196912871185640721060647566693690959);
        ret.lfo_2.set_freq(11.1494673646415847050116189391392436138523280214);
        return ret;
    }

    pub fn set_sr(&mut self, sr: f64) {
        // TODO: make this a thing
        // parent.lfo_1.sr = sr;
        // parent.lfo_2.sr = sr;
    }
}

impl Process<f64> for Dropouts {
    fn step(&mut self, input: f64) -> f64 {
        ((self.lfo_1.step() + self.lfo_2.step()) * 0.5).abs().sqrt().sqrt() /*.sqrt()*/ * input
    }
}