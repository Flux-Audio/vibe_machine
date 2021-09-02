# VIBE_MACHINE v0.1.0

_**Categories:** creative effect, delay, shimmer, lo-fi_

## Installation
_**Disclaimer:** this plugin will only work on 64-bit Windows computers!_
Download the `.dll` file in the `bin/` directory and place it into your DAW's VST folder.

## Compiling the source code
_**Note:** you don't need to compile the source code if you just want to use the plugin, just download the .dll._
Make sure you have Cargo installed on your computer (the Rust compiler). Then in the root of the repository run `cargo build`. 
Once Cargo is done building, there should be a `VIBE_MACHINE.dll` file in the newly created `debug/` directory. Place this file into your DAW's VST folder.

## What is VIBE_MACHINE ?
VIBE_MACHINE is my first commission plugin. It was commissioned by [Synes](https://synes.bandcamp.com/), an experimental electronic producer and dear
friend of mine.

In essence VIBE_MACHINE takes any melodic line you feed into it, and turns it into a trippy chopped-up mess. It is made to approximate the sound of tape
loop ambient.

_**Note:** if you like how this description sounds, go right ahead and try it out before reading the rest, kind of the point of this plugin is that it is
a bit obscure on purpose, and reading how it works might be a bit of a spoiler_

At its core are three delay lines, each of a
different length, which is an irrational relationship compared to the other two delay lines, so that no matter how hard you try, it is impossible to sync
all delay lines with the tempo. Why? Because the whole design philosophy of this plugin is to make you listen to what each knob does and let the plugin's
sound speak for itself.

In addition to being a delay, it has a tape emulation with wow, flutter, dropout artifacts, high frequency loss, hysteresis and saturation, all parametrized into
three controls: vibe, age and tone. It has pitch shifting, which moves each read head of the delay lines along the delay buffer, pitch shifting the signal and
creating chopped-up artifacts when the read head reaches the end of the "tape" and loops over.

There is an additional distortion control in the feedback loop, and a feedback control which goes well above unity gain and drives a CMOS-style soft-clipper when it
approaches 0dB, to avoid breaking your speakers and to add even more distortion if you really want that.

## Controls explained
_**Note:** Each control does a lot of different things in the background, and you are not meant to use this plugin with a technical mindset, with that in mind, here is
my walkthrough of each control, I advise you to only read this after doing your own exploration first_
- time: controls each of the three delay line's length, the range of the center most delay line is from 50 ms to 5000 ms, the other two delay lines, panned hard left and
  right, have a relationship of log2(e) and log2(e)^2 respectively to the center delay length.
  Additionally, when the pitch shifting is enabled, the position of the read head is changing all the time, so these times are not followed strictly anymore, although the
  length still defines the maximum distance that the head will travel before looping back around.
- vibe: adds tape wow emulation, the shape of the tape wow LFO is a sine raised to the fourth power, which has a similar shape as the modulation in a univibe pedal, and 
  each delay line has a separate irrational frequency relationship to the others, creating a thick chorusy effect.
- age: adds flutter and dropout artifacts (short drops in volume), the dropouts come in linearly, while the flutter comes in only very subtly in the first half of the control
  and then becomes quite extreme from the halfway point and up. The flutter is modelled with five stacked sine LFO's all running at slightly different speeds, but much higher
  than the wow LFOs, and five stacked "spike" generators that create a sort of fractal spiky noise signal, meant to simulate the sound of the tape slowing down and speeding up
  again as a piece of dust or creases on the tape pass through the rotors.
- tone: tone is a 1-pole lowpass filter in the feedback path, it has a very gentle slope so its not very noticeable unless it is set quite low, but it adds to the warm and slightly
  dark tone of tape emulation.
- pitch: selects between 13 different presets of pitch shifting, which are all permutations of shifting the left and right channels up or down by a perfect fifth or an octave. In
  the centermost position (preset 0, 0) pitch shifting is disabled and the position of the delay line read heads is reset to their "correct" value.
- feedback: feeds back the signal onto itself, passing through some very mild filtering to remove DC offsets or extremely high frequency resonances that might accumulate. It goes
  well above unity gain, and has a fairly hard CMOS-style clipper to prevent it from shooting above 0dB, but also to add some hard distortion if needed.
- distortion: drives a separate sigmoid saturation section for the left and right channels (which smashes together the center channel with the left and right channels, allowing
  intermodulation to happen across the left and right delay lines). A sigmoid saturation is simply a saturation function in the shape of an S, like a tanh() function or a FET-style
  transistor saturation.
- moisture: a funny name for a dry/wet control.

## Known bugs
For a detailed list, see the [issues](https://github.com/PanieriLorenzo/vibe_machine/issues) tab.

(No bugs 😎😎😎💯🔥 but probably not for long 😧😰😨😱😭)




