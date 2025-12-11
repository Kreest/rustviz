use femtovg::Path;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::scaling::divide_by_N_sqrt;

pub fn plot_wave<S: Iterator<Item = i16>>(
    rendered_samples: S,
    canvas_width: f32,
    canvas_height: f32,
) -> (Path, Path) {
    let mut p1 = Path::new();
    let p1_start = canvas_height / 2.0 - canvas_height / 4.0;
    p1.move_to(0.0, p1_start);
    let mut p2 = Path::new();
    let p2_start = canvas_height / 2.0 + canvas_height / 4.0;
    p2.move_to(0.0, p2_start);
    for (i, sample) in rendered_samples.enumerate().take(40000) {
        if i % 2 == 0 {
            p1.line_to(
                (i as f32) * canvas_width / 40000.,
                (sample as f32 / 500.0) + p1_start,
            );
        } else {
            p2.line_to(
                (i as f32) * canvas_width / 40000.,
                (sample as f32 / 500.0) + p2_start,
            );
        }
    }

    (p1, p2)
}

pub fn plot_freq_spectrum<S: Iterator<Item = i16>>(
    rendered_samples: S,
    canvas_width: f32,
    canvas_height: f32,
) -> Path {
    let mut p1 = Path::new();
    let channel_one_samples = rendered_samples
        .step_by(2)
        .map(|x| x.into())
        .collect::<Vec<_>>();
    if channel_one_samples.len() < 22050 {
        return p1;
    }

    let hann_window = hann_window(&channel_one_samples[0..2_usize.pow(14)]);
    let spectrum_hann_window = samples_fft_to_spectrum(
        // (windowed) samples
        &hann_window,
        // sampling rate
        44100,
        // optional frequency limit: e.g. only interested in frequencies 50 <= f <= 150?
        FrequencyLimit::All,
        // optional scale
        Some(&divide_by_N_sqrt),
    ).unwrap();


    let p1_start = canvas_height / 2.0;
    p1.move_to(0.0, p1_start);
    for (i, sample) in spectrum_hann_window.data().iter() {
        p1.line_to(i.val() * canvas_width / spectrum_hann_window.data().len() as f32, -sample.val() / 2000. + p1_start);
    }

    p1
}
