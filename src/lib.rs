use smithay::{
    backend::{
        input::{Device, DeviceCapability, MouseButton},
        renderer::{
            gles2::{Gles2Frame, Gles2Renderer},
            Frame,
        },
    },
    desktop::space::RenderZindex,
    utils::{Logical, Physical, Rectangle, Size},
    wayland::seat::{Keysym, ModifiersState},
};

#[cfg(feature = "render_element")]
use smithay::{
    backend::renderer::gles2::{Gles2Error, Gles2Texture},
    desktop::space::{RenderElement, SpaceOutputTuple},
    utils::Point,
};

#[cfg(feature = "render_element")]
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

#[cfg(feature = "render_element")]
static EGUI_ID: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "render_element")]
lazy_static::lazy_static! {
    static ref EGUI_IDS: Mutex<HashSet<usize>> = Mutex::new(HashSet::new());
}
#[cfg(feature = "render_element")]
fn next_id() -> usize {
    let mut ids = EGUI_IDS.lock().unwrap();
    debug_assert!(ids.len() != usize::MAX);
    let mut id = EGUI_ID.fetch_add(1, Ordering::SeqCst);
    while ids.iter().any(|k| *k == id) {
        id = EGUI_ID.fetch_add(1, Ordering::SeqCst);
    }

    ids.insert(id);
    id
}

/// Global smithay-egui state
pub struct WallpaperState {
    id: usize,
}

/// A single rendered egui interface frame
pub struct WallpaperFrame {
    state_id: usize,
    area: Rectangle<i32, Physical>,
    size: Size<i32, Physical>,
}

impl WallpaperState {
    /// Creates a new `WallpaperState`
    pub fn new() -> Self {
        Self { id: next_id() }
    }

    /// Produce a new frame of egui to draw onto your output buffer.
    ///
    /// - `ui` is your drawing function
    /// - `area` limits the space egui will be using.
    /// - `size` has to be the total size of the buffer the ui will be displayed in
    /// - `scale` is the scale egui should render in
    /// - `start_time` need to be a fixed point in time before the first `run` call to measure animation-times and the like.
    /// - `modifiers` should be the current state of modifiers pressed on the keyboards.
    pub fn run(
        &mut self,
        area: Rectangle<i32, Logical>,
        size: Size<i32, Physical>,
    ) -> WallpaperFrame {
        let area = area.to_physical(1);
        WallpaperFrame {
            state_id: self.id,
            area,
            size,
        }
    }
}

impl WallpaperFrame {
    /// Draw this frame in the currently active GL-context
    pub unsafe fn draw(&self, r: &mut Gles2Renderer, frame: &Gles2Frame) -> Result<(), Gles2Error> {
        r.with_context(|r, gl| unsafe {
            let transform = frame.transformation();

            Ok(())
        })
        .and_then(std::convert::identity)
    }
}

#[cfg(feature = "render_element")]
impl RenderElement<Gles2Renderer, Gles2Frame, Gles2Error, Gles2Texture> for WallpaperFrame {
    fn id(&self) -> usize {
        self.state_id
    }

    fn geometry(&self) -> Rectangle<i32, Logical> {
        Rectangle::<i32, Logical>::from_loc_and_size((0, 0), (0, 0))
    }

    fn accumulated_damage(
        &self,
        _for_values: Option<SpaceOutputTuple<'_, '_>>,
    ) -> Vec<Rectangle<i32, Logical>> {
        vec![]
    }

    fn draw(
        &self,
        renderer: &mut Gles2Renderer,
        frame: &mut Gles2Frame,
        _scale: f64,
        _damage: &[Rectangle<i32, Logical>],
        log: &slog::Logger,
    ) -> Result<(), Gles2Error> {
        if let Err(err) = unsafe { WallpaperFrame::draw(self, renderer, frame) } {
            slog::error!(log, "egui rendering error: {}", err);
        }
        Ok(())
    }

    fn z_index(&self) -> u8 {
        0
    }
}
