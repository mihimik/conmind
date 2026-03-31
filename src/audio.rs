use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use realfft::RealFftPlanner;

use std::sync::{Arc, Mutex};

pub struct AudioData {
    pub spectrum: Vec<f32>,
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
}

pub fn setup_audio() -> (Arc<Mutex<AudioData>>, cpal::Stream) {
    let host = cpal::default_host();

    let device = host.default_output_device()
        .expect("Unable to find default device output");

    println!("Using device: {}", device.name().unwrap());

    let config = device.default_output_config().expect("Output config error");

    let data = Arc::new(Mutex::new(AudioData {
        spectrum: vec![0.0; 512],
        bass: 0.0, mid: 0.0, high: 0.0,
    }));

    let audio_data_clone = Arc::clone(&data);

    let mut planner = RealFftPlanner::<f32>::new();
    let n = 1024;
    let r2c = planner.plan_fft_forward(n);

    let stream = device.build_input_stream(
        &config.into(),
        move |input_data: &[f32], _: &_| {
            let mut shared = audio_data_clone.lock().unwrap();

            let mut out_data = r2c.make_output_vec();
            let mut in_data = vec![0.0; n];
            let len_to_copy = input_data.len().min(n);
            in_data[..len_to_copy].copy_from_slice(&input_data[..len_to_copy]);
            r2c.process(&mut in_data, &mut out_data).unwrap();

            let mut spectrum: Vec<f32> = out_data.iter().map(|c| c.norm()).collect();

            let len = spectrum.len();
            let gain = 0.04;
            shared.bass = (spectrum[0..5].iter().sum::<f32>() / 5.0) * gain;
            shared.mid = (spectrum[5..50].iter().sum::<f32>() / 45.0) * gain * 2.0;
            shared.high = (spectrum[50..200].iter().sum::<f32>() / 150.0) * gain * 0.2;
        },

        |err| eprintln!("Audio error: {}", err),
        None
    ).unwrap();

    stream.play().unwrap();
    (data, stream)
}

