#[macro_use]
extern crate vst;

#[macro_use]
extern crate rust_dsp_utils;

#[macro_use]
extern crate dsp_lab;

// third-party libs
use vst::buffer::AudioBuffer;
use vst::editor::Editor;
use vst::plugin::{Category, Info, Plugin, PluginParameters, CanDo};
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
use std::collections::HashMap;

// internal dependencies
mod process;
mod logger;
mod editor;
mod algo;
mod widgets;
use crate::logger::Logger;
use crate::editor::EffectEditor;
use crate::algo::Dropouts;

// === GLOBALS ===
const DEBUG_LOGGING_ENABLED: bool = true;

// === PARAMETERS ===
pub struct EffectParameters {
    dict: HashMap<i32, AtomicFloat>,
}

impl Default for EffectParameters {
    fn default() -> Self {
        let mut ret = Self {
            dict: HashMap::new(),
        };
        ret.dict.insert(0, AtomicFloat::new(0.25));   // time
        ret.dict.insert(1, AtomicFloat::new(0.0 ));   // vibe
        ret.dict.insert(2, AtomicFloat::new(0.0 ));   // age
        ret.dict.insert(3, AtomicFloat::new(0.8 ));   // tone
        ret.dict.insert(4, AtomicFloat::new(0.5 ));   // pitch mode
        ret.dict.insert(5, AtomicFloat::new(0.0 ));   // feedback
        ret.dict.insert(6, AtomicFloat::new(0.0 ));   // saturation
        ret.dict.insert(7, AtomicFloat::new(0.5 ));   // dry / wet

        return ret;
    }
}


impl PluginParameters for EffectParameters {
    // the `get_parameter` function reads the value of a parameter.
    fn get_parameter(&self, index: i32) -> f32 {
        match self.dict.get(&index) {
            Some(p) => p.get(),
            None => 0.0,
        }
    }

    // the `set_parameter` function sets the value of a parameter.
    fn set_parameter(&self, index: i32, val: f32) {
        match self.dict.get(&index) {
            Some(p) => p.set(val),
            None => (),
        };
    }
    
    // This is what will display underneath our control.  We can
    // format it into a string that makes the most since.
    fn get_parameter_text(&self, index: i32) -> String {
        let def_val = &AtomicFloat::new(0.0);
        match index {
            0 => format!("{:.2} seconds", 
                self.dict.get(&0).unwrap().get() * 4.45 + 0.05),
            1 => format!("{:.2}", 
                self.dict.get(&1).unwrap().get()),
            2 => format!("{:.2}", 
                self.dict.get(&2).unwrap().get()),
            3 => format!("{:.2}", 
                self.dict.get(&3).unwrap().get()),
            4 => format!("{}", 
                match (self.dict.get(&4).unwrap().get() * 130.0).round() as u32 {
                    0..=9     => "+7, -12",
                    10..=19   => "-12, -12",
                    20..=29   => "-5, -12",
                    30..=39   => "+12, -12",
                    40..=49   => "-5, -5",
                    50..=59   => "0, -5",
                    60..=69   => "0, 0",
                    70..=79   => "+7, 0",
                    80..=89   => "+7, +7",
                    90..=99   => "+7, -5",
                    100..=109 => "+12, +7",
                    110..=119 => "+12, +12",
                    _         => "+12, -5"
            }),
            5 => format!("{:.2}", 
                self.dict.get(&5).unwrap().get() * 0.95),
            6 => format!("{:.2}", 
                self.dict.get(&6).unwrap().get()),
            7 => format!("{:.2}% wet", 
                self.dict.get(&7).unwrap().get() * 100.0),
            _ => "".to_string(),
        }
    }

    // This shows the control's name.
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "time",
            1 => "vibe",
            2 => "age",
            3 => "tone",
            4 => "pitch mode",
            5 => "feedback",
            6 => "sat",
            7 => "dry / wet",
            _ => "",
        }
        .to_string()
    }
}


// === PLUGIN ===

pub struct Effect {
    // Store a handle to the plugin's parameter object.
    params: Arc<EffectParameters>,

    // store a handle to the GUI
    editor: Option<EffectEditor>,

    // store a handle to the openned log file (None if debugging is disabled)
    logger: Arc<Logger>,

    // meta variables
    sr: f64,
    scale: f64, // scaling factor for sr independence of integrals

    // delay lines
    dly_l: DelayLine,
    dly_r: DelayLine,
    // combs_l: DelayLine,
    // combs_r: DelayLine,

    // wow LFO's
    lfo_1: ParOsc,
    lfo_2: ParOsc,
    lfo_3: ParOsc,
    lfo_4: ParOsc,

    // flutter LFO's
    flut_tri_1: AsymTriOsc,
    flut_tri_2: AsymTriOsc,
    flut_tri_3: AsymTriOsc,
    flut_tri_4: AsymTriOsc,
    flut_tri_5: AsymTriOsc,
    flut_sin_1: ParOsc,
    flut_sin_2: ParOsc,
    flut_scrape: NoiseWhite,

    // dropouts
    drop_l: Dropouts,
    drop_r: Dropouts,

    // variable positions
    left_pos:  f64,
    right_pos: f64,

    // filters
    block_dc_l: DcBlock,
    block_dc_r: DcBlock,
    tone_lp_l: LowPass1P,
    tone_lp_r: LowPass1P,

    // param filters
    param_1_lp: LowPass1P,
    param_2_lp: LowPass1P,
    param_3_lp: LowPass1P,
    param_4_lp: LowPass1P,
    param_5_lp: LowPass1P,
    param_6_lp: LowPass1P,
    param_7_lp: LowPass1P,
    param_8_lp: LowPass1P,

    // dithering
    in_dith_l: DenormalDither,
    in_dith_r: DenormalDither,
    fb_dith_l: DenormalDither,
    fb_dith_r: DenormalDither,

    // toneerential variables
    fb_l: f64,
    fb_r: f64,

    // hysteresis
    hyst_l: Hysteresis,
    hyst_r: Hysteresis,
}

impl Default for Effect {
    fn default() -> Effect {
        let params = Arc::new(EffectParameters::default());
        let logger = Arc::new(Logger::new("/flux-audio", "VIBE_MACHINE", DEBUG_LOGGING_ENABLED));
        let mut palette: HashMap<String, (f32, f32, f32, f32)> = HashMap::new();
        palette.insert("knob background".to_string(), (0.2, 0.2, 0.2, 1.0));
        palette.insert("knob fill".to_string(), (0.9, 0.9, 0.9, 1.0));
        Effect {
            // TODO: FIXME: achieve sample rate independence, this requires updating dsp_lab
            // so that things can change their sr after being instantiated.
            params: params.clone(),
            editor: Some(EffectEditor {
                logger: logger.clone(),
                params: params.clone(),
                is_open: false,
                palette: Arc::new(palette),
            }),
            logger: logger.clone(),

            // meta variables
            sr: 44100.0,
            scale: 1.0,

            // delay lines
            dly_l:   DelayLine::new(11000.0, 44100.0, InterpMethod::Quadratic, MixMethod::Average),
            dly_r:   DelayLine::new(11000.0, 44100.0, InterpMethod::Quadratic, MixMethod::Average),
            //combs_l: DelayLine::new(2200.0,  44100.0, InterpMethod::Truncate,  MixMethod::Sqrt),
            //combs_r: DelayLine::new(2200.0,  44100.0, InterpMethod::Truncate,  MixMethod::Sqrt),

            // wow LFO's
            lfo_1: ParOsc::new(0.0, 44100.0),
            lfo_2: ParOsc::new(0.0, 44100.0),
            lfo_3: ParOsc::new(0.0, 44100.0),
            lfo_4: ParOsc::new(0.0, 44100.0),

            // flutter LFO's
            flut_tri_1: AsymTriOsc::new(0.0, 44100.0),
            flut_tri_2: AsymTriOsc::new(0.0, 44100.0),
            flut_tri_3: AsymTriOsc::new(0.0, 44100.0),
            flut_tri_4: AsymTriOsc::new(0.0, 44100.0),
            flut_tri_5: AsymTriOsc::new(0.0, 44100.0),
            flut_sin_1: ParOsc::new(0.0, 44100.0),
            flut_sin_2: ParOsc::new(0.0, 44100.0),
            flut_scrape: NoiseWhite::new(0),

            // dropouts
            drop_l: Dropouts::new(),
            drop_r: Dropouts::new(),

            // variable positions
            left_pos: 0.0,
            right_pos: 0.0,

            // filters
            block_dc_l: DcBlock::new(44100.0),
            block_dc_r: DcBlock::new(44100.0),
            tone_lp_l: LowPass1P::new(44100.0),
            tone_lp_r: LowPass1P::new(44100.0),

            // param filters
            param_1_lp: LowPass1P::new(44100.0),
            param_2_lp: LowPass1P::new(44100.0),
            param_3_lp: LowPass1P::new(44100.0),
            param_4_lp: LowPass1P::new(44100.0),
            param_5_lp: LowPass1P::new(44100.0),
            param_6_lp: LowPass1P::new(44100.0),
            param_7_lp: LowPass1P::new(44100.0),
            param_8_lp: LowPass1P::new(44100.0),

            // dithering
            in_dith_l: DenormalDither::new(3),
            in_dith_r: DenormalDither::new(4),
            fb_dith_l: DenormalDither::new(5),
            fb_dith_r: DenormalDither::new(6),

            // differential variables
            fb_l: 0.0,
            fb_r: 0.0,

            // hysteresis
            hyst_l: Hysteresis::new(),
            hyst_r: Hysteresis::new(),
        }
    }
}

// All plugins using `vst` also need to implement the `Plugin` trait.  Here, we
// define functions that give necessary info to our host.
impl Plugin for Effect {
    fn get_info(&self) -> Info {
        self.logger.log("Plugin::get_info() callback!\n");
        let nfo = Info {
            name: "VIBE_MACHINE".to_string(),
            vendor: "Flux-Audio".to_string(),
            unique_id: 4751486,
            version: 010,
            inputs: 2,
            outputs: 2,
            // This `parameters` bit is important; without it, none of our
            // parameters will be shown!
            parameters: 0,
            category: Category::Effect,
            initial_delay: 0,
            ..Default::default()
        };

        return nfo;
    }

    fn set_sample_rate(&mut self, rate: f32) {
        self.logger.log(&format!("Plugin::set_sample_rate() callback with rate: {}\n", rate)[..]);
        self.sr = rate as f64;
        self.scale = 44100.0 / rate as f64;
    }

    // called once
    fn init(&mut self) {
        self.logger.log("Plugin::init() callback!\n");

        self.dly_l.add_head(500.0, 1.0);
        self.dly_l.add_head(500.0, 1.0);
        self.dly_r.add_head(500.0, 1.0);
        self.dly_r.add_head(500.0, 1.0);

        /*
        for _ in 0..tone_TAPS_L.len(){
            self.combs_l.add_head(500.0, 1.0);
            self.combs_r.add_head(500.0, 1.0);
        }
        */

        // wow LFO's, they all have mutually irrational ratios betweem them, so
        // that they never fully sync up.
        self.lfo_1.set_freq(0.4506093942819681745120095823784220832585749031233);
        self.lfo_2.set_freq(0.6517664324912187283319554965534637881637093311621);
        self.lfo_3.set_freq(0.6224960938630510854555394309830762427824365504454);
        self.lfo_4.set_freq(0.8546512878312836353100107896170289708260075021792);

        // flutter LFO's, they all have mutually irrational ratios between them,
        // so that they never fully sync up.
        // To be precise: 
        // - tri_1 is 0.125 * e/2.7
        // - tri_2 is tri_1 * phi * e/2.7
        // - tri_3 is tri_2 * phi * e/2.7
        // - tri_4 is tri_3 * phi * e/2.7
        // - tri_5 is tri_4 * phi * e/2.7
        // I don't remember where I got the other two.        
        self.flut_tri_1.set_freq(0.8861641217884205093282427772342256043383834);
        self.flut_tri_2.set_freq(0.5439961232435288973996703154361133102831035);
        self.flut_tri_3.set_freq(0.3339469234059613632977919366094348203201041);
        self.flut_tri_4.set_freq(0.2050024676414522912256778335030753190313349);
        self.flut_tri_5.set_freq(0.1258463809471780201555688644144751156369095);
        self.flut_sin_1.set_freq(5.5372407616758321234567890132435842678934068);
        self.flut_sin_2.set_freq(8.9594437562828531234567891011121314151617181);

        // dropout LFO's
        // They are mutually irrational, so they never sync up
        // - drop_1 is  7 * e/2.7
        // - drop_2 is 11 * e/2.7 * e/2.7
        //self.drop_1.set_freq(7.04739733304196912871185640721060647566693690959);
        //self.drop_2.set_freq(11.1494673646415847050116189391392436138523280214);

        // param filters
        // TODO: tune these to maximize sweep speed without artifacts
        self.param_1_lp.set_cutoff(0.5);
        self.param_2_lp.set_cutoff(2.0);
        self.param_3_lp.set_cutoff(7.5);
        self.param_4_lp.set_cutoff(20.0);
        self.param_5_lp.set_cutoff(20.0);
        self.param_6_lp.set_cutoff(20.0);
        self.param_7_lp.set_cutoff(20.0);
        self.param_8_lp.set_cutoff(20.0);
    }

    fn get_editor(&mut self) -> Option<Box<dyn Editor>> {
        self.logger.log("Plugin::get_editor() callback!\n");

        if let Some(editor) = self.editor.take() {
            Some(Box::new(editor) as Box<dyn Editor>)
        } else {
            None
        }
    }

    fn can_do(&self, can_do: CanDo) -> vst::api::Supported {
        self.logger.log("Plugin::can_do() callback!\n");

        use vst::api::Supported::*;
        use vst::plugin::CanDo::*;

        match can_do {
            SendEvents | ReceiveEvents => Yes,
            _ => No,
        }
    }

    // Return the parameter object. This method can be omitted if the
    // plugin has no parameters.
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        self.logger.log("Plugin::get_parameter_object() callback!\n");

        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }

    fn resume(&mut self) {
        self.logger.log("Plugin::resume() callback!\n");
    }

    fn suspend(&mut self) {
        self.logger.log("Plugin::suspend() callback!\n");
    }

    // Here is where the bulk of our audio processing code goes.
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        // === pre-process setup ===
        // TODO: store previous value of flush-to-zero and disable

        process::process_chunk(self, buffer);

        // === post-process cleanup ===
        // TODO: after processing, restore previous value of flush-to-zero
    }
}

plugin_main!(Effect);