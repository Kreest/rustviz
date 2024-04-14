use femtovg::{renderer::OpenGl, Canvas, Color, Paint, Renderer};
use resource::resource;
use ringbuf::{LocalRb, Rb};
use std::fs::File;
use std::sync::Arc;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

mod plotter;
mod window_helper;

use glutin::prelude::*;

use byteorder::{LittleEndian, ReadBytesExt};
use rustfft::FftPlanner;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(EnumIter)]
enum PlotTypes {
    Spectrum,
    Wave,
}

fn main() {
    window_helper::start(1000, 670, false);
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

    let f = File::open("/tmp/mpd.fifo").unwrap();
    let mut reader = f;
    let mut incoming_samples = [0i16; 44100 / 8];
    let mut rendered_samples = LocalRb::<i16, Vec<_>>::new(44100);
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(22050);

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::LoopDestroyed => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    surface.resize(
                        &context,
                        physical_size.width.try_into().unwrap(),
                        physical_size.height.try_into().unwrap(),
                    );
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Space),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    plot_type_cycle.next();
                }
                _ => (),
            },
            Event::RedrawRequested(_) => {
                let res = reader.read_i16_into::<LittleEndian>(&mut incoming_samples);

                match res {
                    Ok(_) => {
                        rendered_samples.push_iter_overwrite(incoming_samples.into_iter());
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }

                let dpi_factor = window.scale_factor();
                let size = window.inner_size();
                canvas.set_size(size.width, size.height, dpi_factor as f32);
                canvas.clear_rect(0, 0, size.width, size.height, Color::rgba(0, 0, 0, 100));

                match plot_type_cycle.peek().unwrap() {
                    PlotTypes::Spectrum => draw_wave(&mut canvas, rendered_samples.iter().cloned()),
                    PlotTypes::Wave => draw_freq_spectrum(
                        &mut canvas,
                        rendered_samples.iter().cloned(),
                        &fft.clone(),
                    ),
                }

                canvas.save();
                canvas.reset();
                canvas.restore();

                canvas.flush();

                surface.swap_buffers(&context).unwrap();
            }
            Event::MainEventsCleared => window.request_redraw(),
            _ => (),
        }
    });
}

fn draw_wave<T: Renderer, S: Iterator<Item = i16>>(canvas: &mut Canvas<T>, rendered_samples: S) {
    let plot_wave = plotter::plot_wave(rendered_samples, canvas.width(), canvas.height());
    let (mut p1, mut p2) = plot_wave;

    canvas.stroke_path(
        &mut p1,
        &Paint::color(Color::rgba(255, 255, 255, 255))
            .with_line_cap(femtovg::LineCap::Round)
            .with_line_join(femtovg::LineJoin::Round)
            .with_line_width(2.0),
    );
    canvas.stroke_path(
        &mut p2,
        &Paint::color(Color::rgba(255, 255, 255, 255))
            .with_line_cap(femtovg::LineCap::Round)
            .with_line_join(femtovg::LineJoin::Round)
            .with_line_width(2.0),
    );
}

fn draw_freq_spectrum<T: Renderer, S: Iterator<Item = i16>>(
    canvas: &mut Canvas<T>,
    rendered_samples: S,
    fft: &Arc<dyn rustfft::Fft<f64>>,
) {
    let mut p1 =
        plotter::plot_freq_spectrum(rendered_samples, fft, canvas.width(), canvas.height());

    canvas.stroke_path(
        &mut p1,
        &Paint::color(Color::rgba(255, 255, 255, 255))
            .with_line_cap(femtovg::LineCap::Round)
            .with_line_join(femtovg::LineJoin::Round)
            .with_line_width(2.0),
    );
}
