#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
extern crate gfx_hal as hal;

use hal::{adapter::PhysicalDevice, queue, Adapter, Gpu, Graphics, Instance, QueueFamily, Surface};
use winit::{dpi::LogicalSize, CreationError, Event, EventsLoop, Window, WindowBuilder, WindowEvent};

pub const WINDOW_NAME: &str = "Hello Triangle";

fn main() {
  env_logger::init();

  let mut winit_state = WinitState::default();

  let instance = back::Instance::create(WINDOW_NAME, 1);
  let surface = instance.create_surface(&winit_state.window);
  let adapter = instance
    .enumerate_adapters()
    .into_iter()
    .filter(|a| {
      a.queue_families
        .iter()
        .find(|qf| qf.supports_graphics() && qf.max_queues() > 0 && surface.supports_queue_family(qf))
        .is_some()
    })
    .next()
    .expect("Couldn't find a graphical Adapter!");
  let (device, command_queues) = {
    let queue_family = adapter
      .queue_families
      .iter()
      .find(|qf| qf.supports_graphics() && qf.max_queues() > 0 && surface.supports_queue_family(qf))
      .expect("Couldn't find a QueueFamily with graphics!");
    let Gpu { device, mut queues } = unsafe {
      adapter
        .physical_device
        .open(&[(&queue_family, &[1.0; 1])])
        .expect("Couldn't open the PhysicalDevice!")
    };
    let queue_group = queues
      .take::<Graphics>(queue_family.id())
      .expect("Couldn't take ownership of the QueueGroup!");
    debug_assert!(queue_group.queues.len() > 0);
    (device, queue_group.queues)
  };

  let mut running = true;
  while running {
    winit_state.events_loop.poll_events(|event| match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => running = false,
      _ => (),
    });
  }
}

#[derive(Debug)]
pub struct WinitState {
  pub events_loop: EventsLoop,
  pub window: Window,
}
impl WinitState {
  /// Constructs a new `EventsLoop` and `Window` pair.
  ///
  /// The specified title and size are used, other elements are default.
  /// ## Failure
  /// It's possible for the window creation to fail. This is unlikely.
  pub fn new<T: Into<String>>(title: T, size: LogicalSize) -> Result<Self, CreationError> {
    let events_loop = EventsLoop::new();
    let output = WindowBuilder::new().with_title(title).with_dimensions(size).build(&events_loop);
    output.map(|window| Self { events_loop, window })
  }
}
impl Default for WinitState {
  /// Makes an 800x600 window with the `WINDOW_NAME` value as the title.
  /// ## Panics
  /// If a `CreationError` occurs.
  fn default() -> Self {
    Self::new(WINDOW_NAME, LogicalSize { width: 800.0, height: 600.0 }).expect("Could not create a window!")
  }
}
