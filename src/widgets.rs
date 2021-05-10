// third-party dependencies
use vst::util::AtomicFloat;
use femtovg::{renderer::OpenGl, Canvas, LineCap, Paint, Path, Solidity, Color, LineJoin};

use tuix::*;

// stl dependencies
use std::sync::Arc;
use std::collections::HashMap;

// internal dependencies
use super::EffectParameters;

/// Set the width and position of the knob with inline properties!
pub struct CustomKnob {
    // inherits mouse listening logic from Tuix ControlKnob
    control: ControlKnob,

    // has a reference to plugin parameters, with a key to access a specific
    // parameter (FIXME: tight coupling, not good, but also dunno what to do
    // about it.
    params: Arc<EffectParameters>,
    dict_key: i32,

    // this gets updated every time the UI is updated, with the last value of
    // the plugin parameter it is referencing.
    last_value: f32,

    // color palette for knob, note that the back_col refers to the background
    // inside the knob fader, the background behind the text is transparent.
    back_col  : Entity,
    main_col  : Entity,     // for borders and text
    fill_col_1: Entity,     // first color of the gradient fill
    fill_col_2: Entity,     // middle color of the gradient fill
    fill_col_3: Entity,     // last color of the gradient fill

    // labels, split in two lines
    label_1: String,
    label_2: String,
    
    // TODO: add entities for font selection, this will have to wait until font
    // properties are implemented in Tuix

    // font paths, relative to rust project root
    label_font:   String,
    readout_font: String,
}

impl CustomKnob {
    pub fn new(params: Arc<EffectParameters>, 
               dict_key: i32, 
               label_1: String,
               label_2: String,
               label_font: String,
               readout_font: String,
            ) -> Self {
        CustomKnob {
            control: ControlKnob::new(0.0, 0.0, 1.0),
            params: params,
            dict_key: dict_key,
            last_value: 0.0,
            back_col: Entity::null(),
            main_col: Entity::null(),
            fill_col_1: Entity::null(),
            fill_col_2: Entity::null(),
            fill_col_3: Entity::null(),
            label_1: label_1,
            label_2: label_2,
            label_font: label_font,
            readout_font: readout_font,
        }
    }
}

impl Widget for CustomKnob {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        
        let value = self.params.dict.get(&self.dict_key).unwrap().get();
        self.control.on_build(state, entity);
        self.last_value    = value;
        self.control.value = value;

        // use widgets to store colors that can be referenced in CSS theme
        self.back_col = Element::new().build(state, entity, |builder| {
            builder
                .set_hoverability(false)
                .set_display(Display::None)
                .class("back_col")
        });
        self.main_col = Element::new().build(state, entity, |builder| {
            builder
                .set_hoverability(false)
                .set_display(Display::None)
                .class("main_col")
        });
        self.fill_col_1 = Element::new().build(state, entity, |builder| {
            builder
                .set_hoverability(false)
                .set_display(Display::None)
                .class("fill_col_1")
        });
        self.fill_col_2 = Element::new().build(state, entity, |builder| {
            builder
                .set_hoverability(false)
                .set_display(Display::None)
                .class("fill_col_2")
        });
        self.fill_col_3 = Element::new().build(state, entity, |builder| {
            builder
                .set_hoverability(false)
                .set_display(Display::None)
                .class("fill_col_3")
        });

        entity.class(state, "custom_knob");
        return entity;
    }

    fn on_event(&mut self, _state: &mut State, _entity: Entity, event: &mut Event) {

        if let Some(slider_event) = event.message.downcast::<SliderEvent>() {
            match slider_event {
                SliderEvent::ValueChanged(val) => {
                    self.last_value = *val;
                    self.params.dict.get(&self.dict_key).unwrap().set(*val);
                }

                _ => {}
            }
        }
    }

    fn on_draw(&mut self, state: &mut State, entity: Entity, canvas: &mut Canvas<OpenGl>) {

        // end early if invisible, skips all rendering (near zero cost)
        if state.data.get_visibility(entity) == Visibility::Invisible { return; }

        let opacity = state.data.get_opacity(entity);

        // background color of the knob
        let mut back_col: Color = self.back_col.get_background_color(state).into();
        back_col.set_alphaf(back_col.a * opacity);

        // main color of the knob, used for text and borders
        let mut main_col: Color = self.main_col.get_background_color(state).into();
        main_col.set_alphaf(main_col.a * opacity);

        // fill colors of the knob, used for the gradient in the color bar
        let mut fill_col_1: Color = self.fill_col_1.get_background_color(state).into();
        fill_col_1.set_alphaf(fill_col_1.a * opacity);
        let mut fill_col_2: Color = self.fill_col_2.get_background_color(state).into();
        fill_col_2.set_alphaf(fill_col_2.a * opacity);
        let mut fill_col_3: Color = self.fill_col_3.get_background_color(state).into();
        fill_col_3.set_alphaf(fill_col_3.a * opacity);

        // calculating fixed geometry
        let posx = state.data.get_posx(entity);
        let posy = state.data.get_posy(entity);
        let width = state.data.get_width(entity);
        let label_1_x = posx + 0.1 * width;
        let label_1_y = posy + 0.2 * width;
        let label_1_width = 0.8 * width;
        let label_1_height = 0.2 * width;
        let label_2_x = label_1_x;
        let label_2_y = posy + 0.4 * width;
        let label_2_width = 0.5 * width;
        let label_2_height = label_1_height;
        let readout_x = posx + 0.6 * width;
        let readout_y = label_2_y;
        let readout_width = 0.3 * width;
        let readout_height = label_1_height;
        let bar_x = label_1_x;
        let bar_y = posy + 0.6 * width;
        let bar_size = label_1_width;
        let gradient_start_x = label_1_x;
        let gradient_start_y = posy + 1.4 * width;
        let gradient_end_x = posx + 0.9 * width;
        let gradient_end_y = bar_y;

        // calculate dynamic geometry (fill rect)
        let fill_x = bar_x;
        let fill_y = bar_y + (1.0 - self.last_value) * bar_size;
        let fill_width = bar_size;
        let fill_height = self.last_value * bar_size;

        /*
        use std::f32::consts::PI;
        let start = -(PI + PI / 4.0);
        let end = PI / 4.0;
        */

        // begin rendering
        canvas.save();

        // draw knob bar
        let mut path = Path::new();
        path  .rect(bar_x, bar_y, bar_size, bar_size);
        let mut paint = Paint::color(back_col);
        canvas.fill_path(&mut path, paint);
        let mut paint = Paint::color(main_col);
        paint .set_line_width(1.0);
        paint .set_line_cap(LineCap::Square);
        canvas.stroke_path(&mut path, paint);

        /*
        let mut path = Path::new();
        path.arc(cx, cy, r1 - 2.5, end, start, Solidity::Solid);
        let mut paint = Paint::color(back_col_fem);
        paint.set_line_width(5.0);
        paint.set_line_cap(LineCap::Butt);
        canvas.stroke_path(&mut path, paint);

        if current != zero_position {
            let mut path = Path::new();
            if current > zero_position {
                path.arc(cx, cy, r1 - 2.5, current, zero_position, Solidity::Solid);
            } else {
                path.arc(cx, cy, r1 - 2.5, zero_position, current, Solidity::Solid);
            }

            let mut paint = Paint::color(fill_col_fem);
            paint.set_line_width(5.0);
            paint.set_line_cap(LineCap::Butt);
            canvas.stroke_path(&mut path, paint);
        }

        // Draw knob
        let mut path = Path::new();
        path.circle(cx, cy, r0 + 1.0);
        let paint = Paint::color(back_col_fem);
        canvas.fill_path(&mut path, paint);

        // Draw knob tick
        canvas.save();
        canvas.translate(cx, cy);
        canvas.rotate(current - PI / 2.0);

        let mut path = Path::new();
        path.circle(0.0, r0 - 2.5, 2.0);
        let paint = Paint::color(fill_col_fem);
        canvas.fill_path(&mut path, paint);

        canvas.restore();
        */
        canvas.restore();
    }
}