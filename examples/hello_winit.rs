#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
extern crate gfx_hal as hal;

use winit::{dpi, ControlFlow, Event, EventsLoop, Window, WindowBuilder, WindowEvent};

static WINDOW_NAME: &str = "Learn gfx-hal: Opening A Window";

fn main() {
  let mut application = WindowApp::init();
  application.run();
  application.clean_up();
}

struct WindowState {
  events_loop: EventsLoop,
  _window: Window,
}

struct HalState {}

impl HalState {
  fn clean_up(self) {}
}

struct WindowApp {
  hal_state: HalState,
  window_state: WindowState,
}

impl WindowApp {
  pub fn init() -> Self {
    let window_state = Self::init_window();
    let hal_state = Self::init_hal();

    Self { hal_state, window_state }
  }

  fn init_window() -> WindowState {
    let events_loop = EventsLoop::new();
    let window = WindowBuilder::new()
      .with_dimensions(dpi::LogicalSize::new(1024., 768.))
      .with_title(WINDOW_NAME)
      .build(&events_loop)
      .expect("Could not create a window!");
    WindowState {
      events_loop,
      _window: window,
    }
  }

  fn init_hal() -> HalState {
    HalState {}
  }

  fn main_loop(&mut self) {
    self.window_state.events_loop.run_forever(|event| match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => ControlFlow::Break,
      _ => ControlFlow::Continue,
    });
  }

  fn run(&mut self) {
    self.main_loop();
  }

  fn clean_up(self) {
    self.hal_state.clean_up();
  }
}
