#![feature(thread_is_running)]

#[cfg(feature = "async_loading")]
use std::thread;

use std::cell::RefCell;
use std::rc::Rc;

use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageResult};
use smithay::utils::Transform;
use smithay::{
    backend::renderer::{
        gles2::{Gles2Frame, Gles2Renderer},
        Frame,
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

mod tools;

#[cfg(feature = "render_element")]
static WALLPAPER_ID: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "render_element")]
lazy_static::lazy_static! {
    static ref WALLPAPER_IDS: Mutex<HashSet<usize>> = Mutex::new(HashSet::new());
}
#[cfg(feature = "render_element")]
fn next_id() -> usize {
    let mut ids = WALLPAPER_IDS.lock().unwrap();
    debug_assert!(ids.len() != usize::MAX);
    let mut id = WALLPAPER_ID.fetch_add(1, Ordering::SeqCst);
    while ids.iter().any(|k| *k == id) {
        id = WALLPAPER_ID.fetch_add(1, Ordering::SeqCst);
    }

    ids.insert(id);
    id
}

/// Global smithay-egui state
#[derive(Debug, Default)]
pub struct WallpaperState {
    id: usize,
    #[cfg(feature = "async_loading")]
    join: Option<thread::JoinHandle<ImageResult<DynamicImage>>>,
    image: Rc<Option<DynamicImage>>,
    texture: Rc<RefCell<Option<Gles2Texture>>>,
}

/// A single rendered egui interface frame
pub struct WallpaperFrame {
    state_id: usize,
    area: Rectangle<i32, Physical>,
    size: Size<i32, Physical>,
    image: Rc<Option<DynamicImage>>,
    texture: Rc<RefCell<Option<Gles2Texture>>>,
}

impl WallpaperState {
    /// Creates a new `WallpaperState`
    pub fn new() -> Self {
        Self {
            id: next_id(),
            ..Default::default()
        }
    }

    #[cfg(feature = "async_loading")]
    fn check(&mut self) {
        if let Some(join) = self.join.take() {
            if !join.is_running() {
                if let Ok(Ok(image)) = join.join() {
                    self.image = Rc::new(Some(image))
                } else {
                    println!("error loading image");
                }
            } else {
                self.join = Some(join);
            }
        }
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

        #[cfg(feature = "async_loading")]
        self.check();

        WallpaperFrame {
            state_id: self.id,
            area,
            size,
            image: self.image.clone(),
            texture: self.texture.clone(),
        }
    }

    #[cfg(feature = "async_loading")]
    pub fn set<P: AsRef<Path>>(&mut self, path: P) {
        let path = PathBuf::from(path.as_ref());
        self.join = Some(thread::spawn(move || image::open(path)));
    }
}

impl WallpaperFrame {
    /// Draw this frame in the currently active GL-context
    pub fn draw(&self, r: &mut Gles2Renderer, frame: &mut Gles2Frame) -> Result<(), Gles2Error> {
        if let Some(image) = &*self.image {
            let mut cached_texture = self.texture.borrow_mut();
            let texture;

            if let Some(cached_texture) = &*cached_texture {
                texture = cached_texture.clone();
            } else {
                *cached_texture =
                    Some(tools::import_bitmap(r, &image.to_rgba8(), self.size.into()).unwrap());
                texture = cached_texture.as_ref().unwrap().clone();
            }

            frame.render_texture_at(
                &texture,
                Point::<i32, Logical>::from((0, 0))
                    .to_f64()
                    .to_physical(1.0)
                    .to_i32_round(),
                1,
                1.0,
                Transform::Normal,
                &[Rectangle::from_loc_and_size((0, 0), (i32::MAX, i32::MAX))],
                1.0,
            )
        } else {
            Ok(())
        }
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
        if let Err(err) = WallpaperFrame::draw(self, renderer, frame) {
            slog::error!(log, "egui rendering error: {}", err);
        }
        Ok(())
    }

    fn z_index(&self) -> u8 {
        0
    }
}
