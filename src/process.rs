// third-party libs
use vst::buffer::AudioBuffer;
use vst::editor::Editor;
use vst::plugin::{Category, Info, Plugin, PluginParameters};
use vst::util::AtomicFloat;

// my own libs
use dsp_lab::core::delay::{DelayLine, MixMethod, InterpMethod};
use dsp_lab::core::lin_filter::{DcBlock, SvfLowPass, LowPass1P};
use dsp_lab::core::chaos::{NoiseWhite, SnhRandom};
use dsp_lab::core::DenormalDither;
use dsp_lab::core::osc::{ParOsc, AsymTriOsc};
use dsp_lab::traits::{Source, Process};
use dsp_lab::emulation::Hysteresis;
use dsp_lab::utils::math::x_fade;

// stl stuff
use std::sync::Arc;
use std::f64::consts;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

// internal dependencies
use super::Effect;


pub fn process_chunk(parent: &mut Effect, buffer: &mut AudioBuffer<f32>) {
    // === get parameters === parameter scaling ===
    let time_raw = parent.params.dict.get(&0).unwrap().get() as f64 * 4450.0 + 50.0;
    let vibe_raw = parent.params.dict.get(&1).unwrap().get()as f64;
    let age_raw = parent.params.dict.get(&2).unwrap().get()  as f64;
    let fb_raw = parent.params.dict.get(&3).unwrap().get() as f64 * 0.95;
    let tone_raw = parent.params.dict.get(&4).unwrap().get() as f64;
    let pitch_mode_raw = (parent.params.dict.get(&5).unwrap().get() * 130.0).round() as u32;
    let sat_raw = parent.params.dict.get(&6).unwrap().get() as f64 * 2.0 + 0.25;
    let wet_raw = parent.params.dict.get(&7).unwrap().get() as f64;

    // === prepare to process chunk ===
    // TODO: enable flush-to-zero and remove all the TINY stuff

    // === process chunk ===
    let (inputs, outputs) = buffer.split();

    // Iterate over inputs as (&f32, &f32)
    let (l, r) = inputs.split_at(1);
    let stereo_in = l[0].iter().zip(r[0].iter());

    // Iterate over outputs as (&mut f32, &mut f32)
    let (mut l, mut r) = outputs.split_at_mut(1);
    let stereo_out = l[0].iter_mut().zip(r[0].iter_mut());

    for ((left_in, right_in), (left_out, right_out)) in stereo_in.zip(stereo_out) {

        // === parameter filtering ===
        let time = parent.param_1_lp.step(time_raw);
        let vibe = parent.param_2_lp.step(vibe_raw);
        let age  = parent.param_3_lp.step(age_raw);
        let fb   = parent.param_4_lp.step(fb_raw);
        let tone = parent.param_5_lp.step(tone_raw * tone_raw * tone_raw * tone_raw);
        let (shift_l, shift_r): (f64, f64) = match pitch_mode_raw {
            0..=9     => ( 0.5,  -0.5),
            10..=19   => (-0.5,  -0.5),
            20..=29   => (-0.25, -0.5),
            30..=39   => ( 1.0,  -0.5),
            40..=49   => (-0.25, -0.25),
            50..=59   => ( 0.0,  -0.25),
            60..=69   => ( 0.0,   0.0),
            70..=79   => ( 0.5,   0.0),
            80..=89   => ( 0.5,   0.5),
            90..=99   => ( 0.5,  -0.25),
            100..=109 => ( 1.0,   0.5),
            110..=119 => ( 1.0,   1.0),
            _         => ( 1.0,  -0.25)
        };
        let sat = parent.param_6_lp.step(sat_raw);
        let wet = parent.param_7_lp.step(wet_raw);

        // === macro mappings ===
        // NOTE: parameters on the UI are macros for a larger set of hidden
        // parameters
        let dry      = 1.0 - wet + 2.0;
        let flutter  = age * age;
        let drop_amt = age;
        let squareness   = 0.8 - sat * 0.8;
        let coercitivity = sat * 0.1;

        // NOTE: what's the deal with all the "TINY"? That's anywhere that
        // a calculation has a potential to give a denormal number as a result.
        // I won't be doing this paranoid treatment on everything, simply
        // because all audio input is passing through a denormal filter
        // anyway, but parameters are not passed through a denormal filter,
        // so they need to be addressed, especially because they are being
        // processed by an IIR filter, which produces denormals as the impulse
        // response approaches zero asymptotically.
        // TODO: HACK: the proper way to deal with denormals is fulsh-to-zero

        // === micro-parameter mappings ===
        // NOTE: a micro-parameter is a parameter passed to low-level processes

        let hyst_l = &mut parent.hyst_l;
        let hyst_r = &mut parent.hyst_r;
        hyst_l.coerc = coercitivity;
        hyst_r.coerc = coercitivity;
        hyst_l.sq = squareness;
        hyst_r.sq = squareness;

        let lfo_1 = parent.lfo_1.step();
        let lfo_2 = parent.lfo_2.step();
        let lfo_3 = parent.lfo_3.step();
        let lfo_4 = parent.lfo_4.step();

        // if pitch shifting is enabled, move read indexes dynamically
        if shift_l != 0.0 || shift_r != 0.0 {
            parent.left_pos -= shift_l / parent.sr * 1000.0;
            if parent.left_pos < 0.0 {
                parent.left_pos += time * consts::LOG2_E;
            }
            if parent.left_pos > time * consts::LOG2_E {
                parent.left_pos -= time * consts::LOG2_E;
            }
            parent.right_pos -= shift_r / parent.sr * 1000.0;
            if parent.right_pos < 0.0{
                parent.right_pos += time * consts::LOG2_E * consts::LOG2_E;
            }
            if parent.right_pos > time * consts::LOG2_E * consts::LOG2_E {
                parent.right_pos -= time * consts::LOG2_E * consts::LOG2_E;
            }
        } else {
            parent.left_pos = time * consts::LOG2_E;
            parent.right_pos = time * consts::LOG2_E * consts::LOG2_E;
        }

        // flutter
        let tri_1 = parent.flut_tri_1.step();
        let tri_2 = parent.flut_tri_2.step();
        let tri_3 = parent.flut_tri_3.step();
        let tri_4 = parent.flut_tri_4.step();
        let tri_5 = parent.flut_tri_5.step();
        let spike_flut = (tri_1 * tri_1 * tri_1 * tri_1 * 0.1
                       + tri_2 * tri_2 * tri_2 * tri_2 * tri_2 * tri_2 * 0.325
                       + tri_3 * tri_3 * tri_3 * tri_3 * tri_3 * tri_3 * tri_3 * tri_3 * 0.55
                       + tri_4 * tri_4 * tri_4 * tri_4 * tri_4 * tri_4 * tri_4 * tri_4 * tri_4 * tri_4 * tri_4 * tri_4 * 0.775
                       + tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5 * tri_5)
                       * 10.0;
        let sin_1 = parent.flut_sin_1.step();
        let sin_2 = parent.flut_sin_2.step();
        let rotor_flut = (sin_1 * sin_1 * sin_1 * sin_1
                       +  sin_2 * sin_2 * sin_2 * sin_2)
                       * 0.333;
        let scrape_flut = parent.flut_scrape.step().abs() * 0.025;
        let total_flut = spike_flut + rotor_flut /* + scrape_flut */;

        let dly_l = &mut parent.dly_l;
        let dly_r = &mut parent.dly_r;
        dly_l.set_offset(0, time           + vibe * lfo_1 * lfo_1 * lfo_1 * lfo_1 *  8.0 + total_flut * flutter);
        dly_l.set_offset(1, parent.left_pos  + vibe * lfo_2 * lfo_2 * lfo_2 * lfo_2 * 10.0 + total_flut * flutter);
        dly_r.set_offset(0, time           + vibe * lfo_3 * lfo_3 * lfo_3 * lfo_3 *  8.0 + total_flut * flutter);
        dly_r.set_offset(1, parent.right_pos + vibe * lfo_4 * lfo_4 * lfo_4 * lfo_4 * 10.0 + total_flut * flutter);

        let drop_l = &mut parent.drop_l;
        let drop_r = &mut parent.drop_r;

        let tone_lp_l = &mut parent.tone_lp_l;
        let tone_lp_r = &mut parent.tone_lp_r;
        tone_lp_l.set_cutoff(tone * 18000.0);
        tone_lp_r.set_cutoff(tone * 18000.0);
    
        // === inputs pre-processing ===
        let mut l = parent.in_dith_l.step(*left_in  as f64) + parent.fb_l;
        let mut r = parent.in_dith_r.step(*right_in as f64) + parent.fb_r;
        let dry_l = l;
        let dry_r = r;

        // === main chain ===
        l = hyst_l.step(l * sat) / sat;
        r = hyst_r.step(r * sat) / sat;
        //l = x_fade(l, tone, combs_l.step(l));
        //r = x_fade(r, tone, combs_r.step(r));
        l = x_fade(l, drop_amt, drop_l.step(l));
        r = x_fade(r, drop_amt, drop_r.step(r));
        l = chain!(l => dly_l => tone_lp_l);
        r = chain!(r => dly_r => tone_lp_r);

        // === feedback chain ===
        let fb_dith_l = &mut parent.fb_dith_l;
        let fb_dith_r = &mut parent.fb_dith_r;
        let block_dc_l = &mut parent.block_dc_l;
        let block_dc_r = &mut parent.block_dc_r;
        parent.fb_l = chain!(l * fb => block_dc_l => fb_dith_l);
        parent.fb_r = chain!(r * fb => block_dc_r => fb_dith_r);

        // === output ===
        *left_out  = (l * wet + dry_l * dry) as f32;
        *right_out = (r * wet + dry_r * dry) as f32;
    }

    // === post-process cleanup ===
    // TODO: after processing, restore previous value of flush-to-zero
}