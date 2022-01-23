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

mod rendering;
mod types;
pub use self::types::{convert_button, convert_key, convert_modifiers};

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
    scale: f64,
    area: Rectangle<i32, Physical>,
    size: Size<i32, Physical>,
}

impl WallpaperState {
    /// Creates a new `WallpaperState`
    pub fn new() -> Self {
        Self { id: next_id() }
    }
}

impl EguiFrame {
    /// Draw this frame in the currently active GL-context
    pub unsafe fn draw(&self, r: &mut Gles2Renderer, frame: &Gles2Frame) -> Result<(), Gles2Error> {
        use rendering::GlState;

        let user_data = r.egl_context().user_data();
        if user_data.get::<GlState>().is_none() {
            let state = GlState::new(r, self.ctx.font_image())?;
            r.egl_context().user_data().insert_if_missing(|| state);
        }

        r.with_context(|r, gl| unsafe {
            let state = r.egl_context().user_data().get::<GlState>().unwrap();
            let transform = frame.transformation();

            state.paint_meshes(
                frame,
                gl,
                self.size,
                self.scale,
                self.mesh
                    .clone()
                    .into_iter()
                    .map(|ClippedMesh(rect, mesh)| {
                        let rect = Rectangle::<f64, Physical>::from_extemities(
                            (rect.min.x as f64, rect.min.y as f64),
                            (rect.max.x as f64, rect.max.y as f64),
                        );
                        let rect = transform.transform_rect_in(rect, &self.size.to_f64());
                        ClippedMesh(
                            Rect {
                                min: (rect.loc.x as f32, rect.loc.y as f32).into(),
                                max: (
                                    (rect.loc.x + rect.size.w) as f32,
                                    (rect.loc.y + rect.size.h) as f32,
                                )
                                    .into(),
                            },
                            mesh,
                        )
                    }),
                self.alpha,
            )
        })
        .and_then(std::convert::identity)
    }
}

#[cfg(feature = "render_element")]
impl RenderElement<Gles2Renderer, Gles2Frame, Gles2Error, Gles2Texture> for EguiFrame {
    fn id(&self) -> usize {
        self.state_id
    }

    fn geometry(&self) -> Rectangle<i32, Logical> {
        let area = self.area.to_f64();

        let used = self.ctx.used_rect();
        Rectangle::<f64, Physical>::from_extemities(
            Point::<f64, Physical>::from((used.min.x as f64 - 30.0, used.min.y as f64 - 30.0))
                + area.loc,
            (used.max.x as f64 + 30.0, used.max.y as f64 + 30.0),
        )
        .to_logical(self.scale)
        .to_i32_round()
    }

    fn accumulated_damage(
        &self,
        _for_values: Option<SpaceOutputTuple<'_, '_>>,
    ) -> Vec<Rectangle<i32, Logical>> {
        if self.output.needs_repaint {
            vec![self.geometry()]
        } else {
            vec![]
        }
    }

    fn draw(
        &self,
        renderer: &mut Gles2Renderer,
        frame: &mut Gles2Frame,
        _scale: f64,
        _damage: &[Rectangle<i32, Logical>],
        log: &slog::Logger,
    ) -> Result<(), Gles2Error> {
        if let Err(err) = unsafe { EguiFrame::draw(self, renderer, frame) } {
            slog::error!(log, "egui rendering error: {}", err);
        }
        Ok(())
    }

    fn z_index(&self) -> u8 {
        0
    }
}
