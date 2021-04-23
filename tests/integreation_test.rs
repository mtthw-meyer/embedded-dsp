use embedded_dsp::*;
use plotters::prelude::*;

const SAMPLE_RATE: u32 = 44100;
const SAMPLE_RATE_F: f32 = 44100.0;

#[test]
fn test_all_pass_phase() {
    const LEN: usize = 512;
    let mut oscillator =
        synthesis::Oscillator::new(synthesis::WaveType::Sine, SAMPLE_RATE_F, 1000.0);
    let mut buffer: [f32; LEN] = [0.0; LEN];
    let delay_line = delay::DelayLine::new(&mut buffer);
    let mut filter = filter::AllPass::new(SAMPLE_RATE_F, delay_line);
    filter.set_freq(1000.0);
    let data = (0..(SAMPLE_RATE)).map(|x| {
        (
            x as f32 / SAMPLE_RATE_F,
            // oscillator.process(),
            filter.process(oscillator.process()),
        )
    });

    let root = BitMapBackend::new("test_all_pass_phase.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .caption("Phase", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..0.02f32, -1.2f32..1.2f32)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    chart.draw_series(LineSeries::new(data, &RED)).unwrap();
}
