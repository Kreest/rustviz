use femtovg::{Canvas, Color, Paint, Renderer, renderer::OpenGl};
use resource::resource;
use ringbuf::{
    storage::Heap,
    traits::{Consumer, RingBuffer},
    LocalRb,
};
use std::io::{self, Read};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

mod plotter;
mod window_helper;

use glutin::prelude::*;

use byteorder::{ByteOrder, LittleEndian};

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use nix::fcntl::OFlag;
use nix::sys::stat::Mode;

#[derive(EnumIter)]
enum PlotTypes {
    Spectrum,
    Wave,
}

fn main() {
    window_helper::start(1000, 670, true);
}

fn run(
    mut canvas: Canvas<OpenGl>,
    el: EventLoop<()>,
    context: glutin::context::PossiblyCurrentContext,
    surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    window: Window,
) {
    canvas
        .add_font_mem(&resource!("assets/Roboto-Regular.ttf"))
        .expect("Cannot add font");

    let mut plot_type_cycle = PlotTypes::iter().cycle().peekable();

    // let f = File::open("/tmp/mpd.fifo").unwrap();
    let fd = nix::fcntl::open(
        "/tmp/mpd.fifo",
        OFlag::O_RDONLY | OFlag::O_NONBLOCK,
        Mode::empty(),
    )
    .expect("open failed");
    let mut reader = std::fs::File::from(fd);
    let mut rendered_samples = LocalRb::<Heap<i16>>::new(44100);

    let mut byte_buf = [0u8; 4096]; // byte buffer
    let _ = el.run(move |event, control_flow| {
        match event {
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    surface.resize(
                        &context,
                        physical_size.width.try_into().unwrap(),
                        physical_size.height.try_into().unwrap(),
                    );
                }
                WindowEvent::CloseRequested => control_flow.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(KeyCode::Space),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    plot_type_cycle.next();
                }
                WindowEvent::RedrawRequested => {
                    match reader.read(&mut byte_buf) {
                        Ok(0) => {
                            // pipe closed
                            return;
                        }
                        Ok(nbytes) => {
                            // Convert the bytes we got into i16 samples
                            let mut samples = Vec::with_capacity(nbytes / 2);

                            for chunk in byte_buf[..nbytes].chunks_exact(2) {
                                samples.push(LittleEndian::read_i16(chunk));
                            }

                            rendered_samples.push_iter_overwrite(samples.into_iter());
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // Ignore these
                        }
                        Err(e) => {
                            println!("Error: {e:?}");
                        }
                    }

                    let dpi_factor = window.scale_factor();
                    let size = window.inner_size();
                    canvas.set_size(size.width, size.height, dpi_factor as f32);
                    canvas.clear_rect(0, 0, size.width, size.height, Color::rgba(0, 0, 0, 100));

                    match plot_type_cycle.peek().unwrap() {
                        PlotTypes::Spectrum => {
                            draw_wave(&mut canvas, rendered_samples.iter().cloned())
                        }
                        PlotTypes::Wave => {
                            draw_freq_spectrum(&mut canvas, rendered_samples.iter().cloned())
                        }
                    }

                    canvas.save();
                    canvas.reset();
                    canvas.restore();

                    canvas.flush();

                    surface.swap_buffers(&context).unwrap();
                }
                _ => (),
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            Event::LoopExiting => control_flow.exit(),
            _ => (),
        }
    });
}

fn draw_wave<T: Renderer, S: Iterator<Item = i16>>(canvas: &mut Canvas<T>, rendered_samples: S) {
    let plot_wave = plotter::plot_wave(
        rendered_samples,
        canvas.width() as f32,
        canvas.height() as f32,
    );
    let (p1, p2) = plot_wave;

    canvas.stroke_path(
        &p1,
        &Paint::color(Color::rgba(255, 255, 255, 255))
            .with_line_cap(femtovg::LineCap::Round)
            .with_line_join(femtovg::LineJoin::Round)
            .with_line_width(2.0),
    );
    canvas.stroke_path(
        &p2,
        &Paint::color(Color::rgba(255, 255, 255, 255))
            .with_line_cap(femtovg::LineCap::Round)
            .with_line_join(femtovg::LineJoin::Round)
            .with_line_width(2.0),
    );
}

fn draw_freq_spectrum<T: Renderer, S: Iterator<Item = i16>>(
    canvas: &mut Canvas<T>,
    rendered_samples: S,
) {
    let p1 = plotter::plot_freq_spectrum(
        rendered_samples,
        canvas.width() as f32,
        canvas.height() as f32,
    );

    canvas.stroke_path(
        &p1,
        &Paint::color(Color::rgba(255, 255, 255, 255))
            .with_line_cap(femtovg::LineCap::Round)
            .with_line_join(femtovg::LineJoin::Round)
            .with_line_width(2.0),
    );
}
