#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;

use gfx_hal::{
  adapter::PhysicalDevice,
  device::Device,
  format::{Aspects, ChannelType, Format, Swizzle},
  image::{Layout, SubresourceRange, ViewKind},
  pass::{Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, Subpass, SubpassDesc},
  pso::{
    BakedStates, BasePipeline, BlendDesc, BlendOp, BlendState, ColorBlendDesc, ColorMask, DepthStencilDesc, DepthTest, DescriptorSetLayoutBinding,
    EntryPoint, Face, Factor, FrontFace, GraphicsPipelineDesc, GraphicsShaderSet, InputAssemblerDesc, LogicOp, PipelineCreationFlags, PolygonMode,
    Rasterizer, Rect, ShaderStageFlags, Specialization, StencilTest, Viewport,
  },
  window::{Backbuffer, Extent2D, SwapchainConfig},
  Backend, Gpu, Graphics, Instance, Primitive, QueueFamily, Surface,
};
use glsl_to_spirv::ShaderType;
use std::{io::Read, ops::Range};
use winit::{dpi::LogicalSize, CreationError, Event, EventsLoop, Window, WindowBuilder, WindowEvent};

pub const WINDOW_NAME: &str = "Hello Triangle";

fn main() {
  env_logger::init();

  let mut winit_state = WinitState::default();

  let instance = back::Instance::create(WINDOW_NAME, 1);
  let mut surface = instance.create_surface(&winit_state.window);
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
  // DESCRIBE
  let (swapchain, extent, backbuffer, format) = {
    let (caps, formats, _present_modes, _composite_alphas) = surface.compatibility(&adapter.physical_device);
    let format = formats.map_or(Format::Rgba8Srgb, |formats| {
      formats
        .iter()
        .find(|format| format.base_format().1 == ChannelType::Srgb)
        .map(|format| *format)
        .unwrap_or(formats[0])
    });
    let swap_config = SwapchainConfig::from_caps(&caps, format, caps.extents.end);
    let extent = swap_config.extent;
    let (swapchain, backbuffer) = unsafe { device.create_swapchain(&mut surface, swap_config, None).unwrap() };
    (swapchain, extent, backbuffer, format)
  };
  // DESCRIBE
  let frame_images: Vec<(<back::Backend as Backend>::Image, <back::Backend as Backend>::ImageView)> = match backbuffer {
    Backbuffer::Images(images) => images
      .into_iter()
      .map(|image| {
        let image_view = unsafe {
          device
            .create_image_view(
              &image,
              ViewKind::D2,
              format,
              Swizzle::NO,
              SubresourceRange {
                aspects: Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
              },
            )
            .expect("Couldn't create the image_view for the image!")
        };
        (image, image_view)
      })
      .collect(),
    Backbuffer::Framebuffer(_) => unimplemented!("Can't handle framebuffer backbuffer!"),
  };
  // DESCRIBE
  let render_pass = {
    let color_attachment = Attachment {
      format: Some(format),
      samples: 1,
      ops: AttachmentOps {
        load: AttachmentLoadOp::Clear,
        store: AttachmentStoreOp::Store,
      },
      stencil_ops: AttachmentOps::DONT_CARE,
      layouts: Layout::Undefined..Layout::Present,
    };
    let subpass = SubpassDesc {
      colors: &[(0, Layout::ColorAttachmentOptimal)],
      depth_stencil: None,
      inputs: &[],
      resolves: &[],
      preserves: &[],
    };
    unsafe {
      device
        .create_render_pass(&[color_attachment], &[subpass], &[])
        .expect("Couldn't create a render pass!")
    }
  };
  // DESCRIBE
  let (descriptor_set_layouts, pipeline_layout, gfx_pipeline) = create_graphics_pipeline(&device, extent, &render_pass);

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

  // CLEANUP
  unsafe {
    for (_, image_view) in frame_images.into_iter() {
      device.destroy_image_view(image_view);
    }
    device.destroy_swapchain(swapchain);
  };
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

pub fn create_graphics_pipeline(
  device: &<back::Backend as Backend>::Device, extent: Extent2D, render_pass: &<back::Backend as Backend>::RenderPass,
) -> (
  Vec<<back::Backend as Backend>::DescriptorSetLayout>,
  <back::Backend as Backend>::PipelineLayout,
  <back::Backend as Backend>::GraphicsPipeline,
) {
  let vertex_shader_code = glsl_to_spirv::compile(include_str!("hello_triangle.vert"), ShaderType::Vertex)
    .expect("Error compiling the vertex shader!")
    .bytes()
    .map(Result::unwrap)
    .collect::<Vec<u8>>();
  let fragment_shader_code = glsl_to_spirv::compile(include_str!("hello_triangle.frag"), ShaderType::Fragment)
    .expect("Error compiling the fragment shader!")
    .bytes()
    .map(Result::unwrap)
    .collect::<Vec<u8>>();

  let vertex_shader_module = unsafe {
    device
      .create_shader_module(&vertex_shader_code)
      .expect("Error creating vertex shader module!")
  };
  let fragment_shader_module = unsafe {
    device
      .create_shader_module(&fragment_shader_code)
      .expect("Error creating fragment shader module!")
  };

  let (ds_layouts, pipeline_layout, gfx_pipeline) = {
    let (vs_entry, fs_entry) = (
      EntryPoint::<back::Backend> {
        entry: "main",
        module: &vertex_shader_module,
        specialization: Specialization { constants: &[], data: &[] },
      },
      EntryPoint::<back::Backend> {
        entry: "main",
        module: &fragment_shader_module,
        specialization: Specialization { constants: &[], data: &[] },
      },
    );
    let shaders = GraphicsShaderSet {
      vertex: vs_entry,
      hull: None,
      domain: None,
      geometry: None,
      fragment: Some(fs_entry),
    };
    let rasterizer = Rasterizer {
      depth_clamping: false,
      polygon_mode: PolygonMode::Fill,
      cull_face: Face::BACK,
      front_face: FrontFace::Clockwise,
      depth_bias: None,
      conservative: false,
    };
    let vertex_buffers = vec![];
    let attributes = vec![];
    let input_assembler = InputAssemblerDesc::new(Primitive::TriangleList);
    let blender = {
      let blend_state = BlendState::On {
        color: BlendOp::Add {
          src: Factor::One,
          dst: Factor::Zero,
        },
        alpha: BlendOp::Add {
          src: Factor::One,
          dst: Factor::Zero,
        },
      };
      BlendDesc {
        logic_op: Some(LogicOp::Copy),
        targets: vec![ColorBlendDesc(ColorMask::ALL, blend_state)],
      }
    };
    let depth_stencil = DepthStencilDesc {
      depth: DepthTest::Off,
      depth_bounds: false,
      stencil: StencilTest::Off,
    };
    let multisampling = None;
    let baked_states = BakedStates {
      viewport: Some(Viewport {
        rect: Rect {
          x: 0,
          y: 0,
          w: extent.width as i16,
          h: extent.height as i16,
        },
        depth: (0.0..1.0),
      }),
      scissor: Some(Rect {
        x: 0,
        y: 0,
        w: extent.width as i16,
        h: extent.height as i16,
      }),
      blend_color: None,
      depth_bounds: None,
    };
    let bindings: Vec<DescriptorSetLayoutBinding> = vec![];
    let immutable_samplers: Vec<<back::Backend as Backend>::Sampler> = vec![];
    let ds_layouts = unsafe { vec![device.create_descriptor_set_layout(bindings, immutable_samplers).unwrap()] };
    let push_constants: Vec<(ShaderStageFlags, Range<u32>)> = vec![];
    let pipeline_layout = unsafe { device.create_pipeline_layout(&ds_layouts, push_constants).unwrap() };
    let subpass = Subpass {
      index: 0,
      main_pass: render_pass,
    };
    let flags = PipelineCreationFlags::empty();
    let parent = BasePipeline::None;
    let gfx_pipeline = {
      let desc = GraphicsPipelineDesc {
        shaders,
        rasterizer,
        vertex_buffers,
        attributes,
        input_assembler,
        blender,
        depth_stencil,
        multisampling,
        baked_states,
        layout: &pipeline_layout,
        subpass,
        flags,
        parent,
      };
      unsafe {
        device
          .create_graphics_pipeline(&desc, None)
          .expect("Failed to create a graphics pipeline!")
      }
    };
    (ds_layouts, pipeline_layout, gfx_pipeline)
  };

  unsafe {
    device.destroy_shader_module(vertex_shader_module);
    device.destroy_shader_module(fragment_shader_module);
  }

  (ds_layouts, pipeline_layout, gfx_pipeline)
}
