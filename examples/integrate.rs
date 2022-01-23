use anyhow::Result;
use smithay::{
    backend::{
        renderer::{Frame, Renderer},
        winit,
    },
    reexports::wayland_server::Display,
    utils::{Rectangle, Transform},
    wayland::{
        seat::{FilterResult, ModifiersState, Seat, XkbConfig},
        SERIAL_COUNTER,
    },
};
use smithay_wallpaper::WallpaperState;
use std::cell::RefCell;

// This example provides a minimal example to:
// - Setup and `Renderer` and get `InputEvents` via winit.
// - Pass those input events to egui
// - Render an egui interface using that renderer.

// It does not show of the best practices to do so,
// neither does the example show-case a real wayland compositor.
// For that take a look into [`anvil`](https://github.com/Smithay/smithay/tree/master/anvil).

// This is only meant to provide a starting point to integrate egui into an already existing compositor

fn main() -> Result<()> {
    // setup logger
    let _guard = setup_logger();
    // create a winit-backend
    let (mut backend, mut input) = winit::init(None)?;
    // create an `EguiState`. Usually this would be part of your global smithay state
    let mut wallpaper = WallpaperState::new();
    // this is likely already part of your ui-state for `send_frames` and similar
    let start_time = std::time::Instant::now();
    // We need to track the current set of modifiers, because egui expects them to be passed for many events
    let modifiers = RefCell::new(ModifiersState::default());

    // Usually you should already have a seat
    let mut display = Display::new();
    let (mut seat, _global) = Seat::new(&mut display, "seat-0".to_string(), None);
    // For a real compositor we would add a socket here and put the display inside an event loop,
    // but all we need for this example is the seat for it's input handling
    let keyboard = seat.add_keyboard(XkbConfig::default(), 200, 25, |_seat, _focus| {})?;

    loop {
        input.dispatch_new_events(|event| {
            use smithay::backend::{
                input::{
                    Axis, ButtonState, Event, InputEvent, KeyState, KeyboardKeyEvent,
                    PointerAxisEvent, PointerButtonEvent, PointerMotionAbsoluteEvent,
                },
                winit::WinitEvent::*,
            };
            match event {
                Input(event) => match event {
                    _ => {}
                },
                _ => {}
            }
        })?;

        let size = backend.window_size().physical_size;

        // Here we compute the rendered egui frame
        let wallpaper_frame = wallpaper.run(
            // Just render it over the whole window, but you may limit the area
            Rectangle::from_loc_and_size((0, 0), size.to_logical(1)),
            size,
        );

        // Lastly put the rendered frame on the screen
        backend.bind()?;
        let renderer = backend.renderer();
        renderer
            .render(size, Transform::Flipped180, |renderer, frame| {
                frame.clear(
                    [1.0, 1.0, 1.0, 1.0],
                    &[Rectangle::from_loc_and_size((0, 0), size)],
                )?;
                unsafe { wallpaper_frame.draw(renderer, frame) }
            })?
            .map_err(|err| anyhow::format_err!("{}", err))?;
        backend.submit(None, 1.0)?;
    }
}

fn setup_logger() -> Result<slog_scope::GlobalLoggerGuard> {
    use slog::Drain;

    let decorator = slog_term::TermDecorator::new().stderr().build();
    // usually we would not want to use a Mutex here, but this is usefull for a prototype,
    // to make sure we do not miss any in-flight messages, when we crash.
    let logger = slog::Logger::root(
        std::sync::Mutex::new(
            slog_term::CompactFormat::new(decorator)
                .build()
                .ignore_res(),
        )
        .fuse(),
        slog::o!(),
    );
    let guard = slog_scope::set_global_logger(logger);
    slog_stdlog::init().unwrap();
    Ok(guard)
}
