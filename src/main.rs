// #[cfg(feature = "dx12")]
// use gfx_backend_dx12 as back;
// #[cfg(feature = "metal")]
// use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;

use arrayvec::ArrayVec;
use gfx_hal::adapter::PhysicalDevice;
use gfx_hal::command::{ClearColor, ClearValue, CommandBuffer, MultiShot, Primary};
use gfx_hal::device::Device;
use gfx_hal::format::{Aspects, ChannelType, Format, Swizzle};
use gfx_hal::image::{Extent, Layout, SubresourceRange, Usage, ViewKind};
use gfx_hal::pass::{Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc};
use gfx_hal::pool::CommandPoolCreateFlags;
use gfx_hal::pso::{PipelineStage, Rect};
use gfx_hal::queue::family::QueueFamily;
use gfx_hal::window::Surface;
use gfx_hal::window::Swapchain;
use gfx_hal::{
    Adapter, Backend, CommandPool, CompositeAlpha, Features, Gpu, Graphics, Instance, PresentMode,
    QueueGroup, Submission, SwapchainConfig,
};
use log::{error, info};
use std::mem::ManuallyDrop;
use std::time::Instant;
use winit::dpi::LogicalSize;
use winit::{Event, EventsLoop, Window, WindowBuilder, WindowEvent};

#[derive(Debug)]
struct WinitState {
    events_loop: EventsLoop,
    window: Window,
}

const WINDOW_TITLE: &str = "okno";

impl WinitState {
    fn new<T: Into<String>>(title: T, dimensions: LogicalSize) -> Self {
        let events_loop = EventsLoop::new();
        let window = WindowBuilder::new()
            .with_title(title.into())
            .with_dimensions(dimensions)
            .build(&events_loop)
            .expect("Could not create a window!");

        WinitState {
            events_loop,
            window,
        }
    }
}

impl Default for WinitState {
    fn default() -> Self {
        WinitState::new(WINDOW_TITLE, LogicalSize::new(1600.0, 900.0))
    }
}

#[derive(Debug)]
struct HalState {
    current_frame: usize,
    frames_in_flight: usize,
    in_flight_fences: Vec<<back::Backend as Backend>::Fence>,
    render_finished_semaphores: Vec<<back::Backend as Backend>::Semaphore>,
    image_available_semaphores: Vec<<back::Backend as Backend>::Semaphore>,
    command_buffers: Vec<CommandBuffer<back::Backend, Graphics, MultiShot, Primary>>,
    command_pool: ManuallyDrop<CommandPool<back::Backend, Graphics>>,
    framebuffers: Vec<<back::Backend as Backend>::Framebuffer>,
    image_views: Vec<(<back::Backend as Backend>::ImageView)>,
    render_pass: ManuallyDrop<<back::Backend as Backend>::RenderPass>,
    render_area: Rect,
    queue_group: ManuallyDrop<QueueGroup<back::Backend, Graphics>>,
    swapchain: ManuallyDrop<<back::Backend as Backend>::Swapchain>,
    device: ManuallyDrop<back::Device>,
    _adapter: Adapter<back::Backend>,
    _surface: <back::Backend as Backend>::Surface,
    _instance: ManuallyDrop<back::Instance>,
}

impl HalState {
    fn new(window: &Window) -> Result<Self, &'static str> {
        let instance = back::Instance::create(WINDOW_TITLE, 1);
        let mut surface = instance.create_surface(window);

        let adapter = instance
            .enumerate_adapters()
            .into_iter()
            .find(|it| {
                info!("Found adapter: {} {:?}", it.info.name, it.info.device_type);
                it.queue_families
                    .iter()
                    .any(|qf| qf.supports_graphics() && surface.supports_queue_family(qf))
            })
            .ok_or("No suitable adapter found!")?;

        let (device, queue_group) = {
            let queue_family = &adapter
                .queue_families
                .iter()
                .find(|qf| qf.supports_graphics() && surface.supports_queue_family(qf))
                .ok_or("Couldn't find a QueueFamily with graphics!")?;
            let Gpu { device, mut queues } = unsafe {
                let physical_device = &adapter.physical_device;

                physical_device
                    .open(&[(&queue_family, &[1.0; 1])], Features::empty())
                    .map_err(|_| "Couldn't open the PhysicalDevice!")?
            };
            let queue_group = queues
                .take::<Graphics>(queue_family.id())
                .ok_or("Couldn't take ownership of the QueueGroup!")?;
            let _ = if queue_group.queues.len() > 0 {
                Ok(())
            } else {
                Err("The QueueGroup did not have any CommandQueues available!")
            }?;
            (device, queue_group)
        };

        let (capabilities, formats, present_modes) =
            surface.compatibility(&adapter.physical_device);

        let (swapchain, extent, backbuffer, format, frames_in_flight) = {
            info!("{:?}", capabilities);
            info!("Preferred Formats: {:?}", formats);
            info!("Present Modes: {:?}", present_modes);

            /* Find the best present mode the GPU supports. */
            let present_mode = {
                use gfx_hal::window::PresentMode::*;
                [Mailbox, Fifo, Relaxed, Immediate]
                    .iter()
                    .cloned()
                    .find(|pm| present_modes.contains(pm))
                    .ok_or("No PresentMode values specified!")?
            };

            let format = match formats {
                None => Format::Rgba8Srgb,
                Some(formats) => match formats
                    .iter()
                    .find(|format| format.base_format().1 == ChannelType::Srgb)
                    .cloned()
                {
                    Some(srgb_format) => srgb_format,
                    None => formats
                        .get(0)
                        .cloned()
                        .ok_or("Preferred format list was empty!")?,
                },
            };

            let image_count = if present_mode == PresentMode::Mailbox {
                (capabilities.image_count.end - 1).min(3)
            } else {
                (capabilities.image_count.end - 1).min(2)
            };

            let config = SwapchainConfig {
                present_mode,
                format,
                image_count,
                composite_alpha: CompositeAlpha::OPAQUE,
                extent: capabilities.extents.end,
                image_layers: 1,
                image_usage: Usage::COLOR_ATTACHMENT,
            };
            let (swapchain, backbuffer) = unsafe {
                device
                    .create_swapchain(&mut surface, config, None)
                    .map_err(|_| "Cannot create swapchain!")?
            };

            (
                swapchain,
                capabilities.extents.end,
                backbuffer,
                format,
                image_count as usize,
            )
        };

        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) = {
            let mut image_available_semaphores = vec![];
            let mut render_finished_semaphores = vec![];
            let mut in_flight_fences = vec![];

            for _ in 0..frames_in_flight {
                in_flight_fences.push(
                    device
                        .create_fence(true)
                        .map_err(|_| "Cannot create a fence!")?,
                );
                image_available_semaphores.push(
                    device
                        .create_semaphore()
                        .map_err(|_| "Could not create a semaphore!")?,
                );
                render_finished_semaphores.push(
                    device
                        .create_semaphore()
                        .map_err(|_| "Could not create a semaphore!")?,
                );
            }
            (
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
            )
        };

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

        let render_pass = unsafe {
            device
                .create_render_pass(&[color_attachment], &[subpass], &[])
                .map_err(|_| "Cannot create render pass!")?
        };

        let image_views = backbuffer
            .into_iter()
            .map(|image| unsafe {
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
                    .map_err(|_| "Couldn't create the image_view for the image!")
            })
            .collect::<Result<Vec<_>, &str>>()?;

        let framebuffers: Vec<_> = image_views
            .iter()
            .map(|image_view| unsafe {
                device
                    .create_framebuffer(
                        &render_pass,
                        vec![image_view],
                        Extent {
                            width: extent.width as u32,
                            height: extent.height as u32,
                            depth: 1,
                        },
                    )
                    .map_err(|_| "Failed to create a framebuffer!")
            })
            .collect::<Result<Vec<_>, &str>>()?;

        let mut command_pool = unsafe {
            device
                .create_command_pool_typed(&queue_group, CommandPoolCreateFlags::RESET_INDIVIDUAL)
                .map_err(|_| "Could not create command pool!")?
        };

        let command_buffers: Vec<_> = framebuffers
            .iter()
            .map(|_| command_pool.acquire_command_buffer())
            .collect();

        Ok(Self {
            _instance: ManuallyDrop::new(instance),
            _surface: surface,
            _adapter: adapter,
            device: ManuallyDrop::new(device),
            queue_group: ManuallyDrop::new(queue_group),
            swapchain: ManuallyDrop::new(swapchain),
            render_area: extent.to_extent().rect(),
            render_pass: ManuallyDrop::new(render_pass),
            image_views,
            framebuffers,
            command_pool: ManuallyDrop::new(command_pool),
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            frames_in_flight,
            current_frame: 0,
        })
    }

    pub fn draw_clear_frame(&mut self, color: [f32; 4]) -> Result<(), &'static str> {
        // SETUP FOR THIS FRAME
        let image_available = &self.image_available_semaphores[self.current_frame];
        let render_finished = &self.render_finished_semaphores[self.current_frame];
        // Advance the frame _before_ we start using the `?` operator
        self.current_frame = (self.current_frame + 1) % self.frames_in_flight;

        let (i_u32, i_usize) = unsafe {
            let image_index = self
                .swapchain
                .acquire_image(core::u64::MAX, Some(image_available), None)
                .map_err(|_| "Couldn't acquire an image from the swapchain!")?;
            (image_index.0, image_index.0 as usize)
        };

        let flight_fence = &self.in_flight_fences[i_usize];
        unsafe {
            self.device
                .wait_for_fence(flight_fence, core::u64::MAX)
                .map_err(|_| "Failed to wait on the fence!")?;
            self.device
                .reset_fence(flight_fence)
                .map_err(|_| "Couldn't reset the fence!")?;
        }

        // RECORD COMMANDS
        unsafe {
            let buffer = &mut self.command_buffers[i_usize];
            let clear_values = [ClearValue::Color(ClearColor::Float(color))];
            buffer.begin(false);
            buffer.begin_render_pass_inline(
                &self.render_pass,
                &self.framebuffers[i_usize],
                self.render_area,
                clear_values.iter(),
            );
            buffer.finish();
        }

        // SUBMISSION AND PRESENT
        let command_buffers = &self.command_buffers[i_usize..=i_usize];
        let wait_semaphores: ArrayVec<[_; 1]> =
            [(image_available, PipelineStage::COLOR_ATTACHMENT_OUTPUT)].into();
        let signal_semaphores: ArrayVec<[_; 1]> = [render_finished].into();
        // yes, you have to write it twice like this. yes, it's silly.
        let present_wait_semaphores: ArrayVec<[_; 1]> = [render_finished].into();
        let submission = Submission {
            command_buffers,
            wait_semaphores,
            signal_semaphores,
        };
        let the_command_queue = &mut self.queue_group.queues[0];
        unsafe {
            the_command_queue.submit(submission, Some(flight_fence));
            self.swapchain
                .present(the_command_queue, i_u32, present_wait_semaphores)
                .map_err(|_| "Failed to present into the swapchain!")?;
            Ok(())
        }
    }
}

impl Drop for HalState {
    fn drop(&mut self) {
        self.device.wait_idle().unwrap();
        unsafe {
            for fence in self.in_flight_fences.drain(..) {
                self.device.destroy_fence(fence);
            }

            for semaphore in self.render_finished_semaphores.drain(..) {
                self.device.destroy_semaphore(semaphore)
            }

            for semaphore in self.image_available_semaphores.drain(..) {
                self.device.destroy_semaphore(semaphore)
            }

            for framebuffer in self.framebuffers.drain(..) {
                self.device.destroy_framebuffer(framebuffer);
            }

            for image_view in self.image_views.drain(..) {
                self.device.destroy_image_view(image_view);
            }

            // LAST RESORT STYLE CODE, NOT TO BE IMITATED LIGHTLY
            use core::ptr::read;
            self.device.destroy_command_pool(
                ManuallyDrop::into_inner(read(&self.command_pool)).into_raw(),
            );
            self.device
                .destroy_render_pass(ManuallyDrop::into_inner(read(&self.render_pass)));
            self.device
                .destroy_swapchain(ManuallyDrop::into_inner(read(&self.swapchain)));
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self._instance);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserInput {
    pub end_requested: bool,
    pub new_frame_size: Option<(f64, f64)>,
    pub new_mouse_position: Option<(f64, f64)>,
}

impl UserInput {
    pub fn poll_events_loop(events_loop: &mut EventsLoop) -> Self {
        let mut output = UserInput::default();
        events_loop.poll_events(|event| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => output.end_requested = true,
            Event::WindowEvent {
                event: WindowEvent::Resized(logical),
                ..
            } => {
                output.new_frame_size = Some((logical.width, logical.height));
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                output.new_mouse_position = Some((position.x, position.y));
            }
            _ => (),
        });
        output
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalState {
    pub frame_width: f64,
    pub frame_height: f64,
    pub mouse_x: f64,
    pub mouse_y: f64,
}

impl LocalState {
    pub fn update_from_input(&mut self, input: UserInput) {
        if let Some(frame_size) = input.new_frame_size {
            self.frame_width = frame_size.0;
            self.frame_height = frame_size.1;
        }
        if let Some(position) = input.new_mouse_position {
            self.mouse_x = position.0;
            self.mouse_y = position.1;
        }
    }
}

fn do_the_render(hal_state: &mut HalState, local_state: &LocalState) -> Result<(), &'static str> {
    let r = (local_state.mouse_x / local_state.frame_width) as f32;
    let g = (local_state.mouse_y / local_state.frame_height) as f32;
    let b = (r + g) * 0.3;
    let a = 1.0;
    hal_state.draw_clear_frame([r, g, b, a])
}

fn main() {
    simple_logger::init().unwrap();

    let start = Instant::now();
    let mut winit_state = WinitState::default();
    info!("window created in {}ms", start.elapsed().as_millis());
    let start = Instant::now();

    let mut hal_state = HalState::new(&winit_state.window).expect("cannot create hal");
    info!(
        "vulkan context created in {}ms",
        start.elapsed().as_millis()
    );

    let mut local_state = LocalState::default();

    loop {
        let inputs = UserInput::poll_events_loop(&mut winit_state.events_loop);
        if inputs.end_requested {
            break;
        }
        local_state.update_from_input(inputs);
        if let Err(e) = do_the_render(&mut hal_state, &local_state) {
            error!("Rendering Error: {:?}", e);
            break;
        }
    }
}
