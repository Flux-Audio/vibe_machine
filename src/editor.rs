// third party dependencies
use vst::editor::Editor;

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use tuix::*;

// stl dependencies
use std::sync::Arc;

// internal dependencies
use super::EffectParameters;
use crate::logger::Logger;

// === GLOBALS ===
const WINDOW_WIDTH:  usize = 300;
const WINDOW_HEIGHT: usize = 300;
static THEME: &str = include_str!("theme.css");

// === EDITOR ===
pub struct EffectWidget {
    control: Entity,
    params: Arc<EffectParameters>,
}

impl EffectWidget {
    pub fn new(params: Arc<EffectParameters>) -> Self {
        EffectWidget {
            control: Entity::null(),
            params: params.clone(),
        }
    }
}

impl Widget for EffectWidget {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        
        let val = self.params.dry_wet.get();
        self.control = ValueKnob::new("", val, 0.0, 1.0).build(state, entity, |builder| builder);
        
        self.control
    }

    fn on_event(&mut self, _state: &mut State, _entity: Entity, event: &mut Event) {

        if let Some(slider_event) = event.message.downcast::<SliderEvent>() {
            match slider_event {
                SliderEvent::ValueChanged(val) => {
                    self.params.dry_wet.set(*val);
                }

                _ => {}
            }
        }
    }
}

pub struct EffectEditor {
    pub logger: Arc<Logger>,
    pub params: Arc<EffectParameters>,
    pub is_open: bool,
}

impl Editor for EffectEditor {
    fn position(&self) -> (i32, i32) {
        self.logger.log("Editor::position() callback!\n");
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        self.logger.log("Editor::size() callback!\n");
        (WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32)
    }

    fn open(&mut self, parent: *mut ::std::ffi::c_void) -> bool {
        self.logger.log("[BEGIN] Editor::open()\n");
        if self.is_open {
            self.logger.log(">>> editor is already open.\n");
            self.logger.log("[END] Editor::open()\n");
            return false;
        }

        self.is_open = true;

        let params = self.params.clone();

        self.logger.log(">>> creating window description.\n");
        let window_description = WindowDescription::new()
            .with_title("VIBE_MACHINE")
            .with_inner_size(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32);

            self.logger.log(">>> creating application.\n");
        Application::new(window_description, move |state, window|{
            state.add_theme(THEME);

            EffectWidget::new(params.clone()).build(state, window, |builder| {
                builder
            });
        }).open_parented(&VstParent(parent));
        
        self.logger.log("[END] Editor::open()\n");
        true
    }

    fn is_open(&mut self) -> bool {
        self.logger.log("Editor::is_open() callback!\n");
        self.is_open
    }

    fn close(&mut self) {
        self.logger.log("Editor::close() callback!\n");
        self.is_open = false;
    }
}



// OS-specific raw window handles

struct VstParent(*mut ::std::ffi::c_void);

#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::macos::MacOSHandle;

        RawWindowHandle::MacOS(MacOSHandle {
            ns_view: self.0 as *mut ::std::ffi::c_void,
            ..MacOSHandle::empty()
        })
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::windows::WindowsHandle;

        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.0,
            ..WindowsHandle::empty()
        })
    }
}

#[cfg(target_os = "linux")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::unix::XcbHandle;

        RawWindowHandle::Xcb(XcbHandle {
            window: self.0 as u32,
            ..XcbHandle::empty()
        })
    }
}