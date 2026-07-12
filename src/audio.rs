use std::fs;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, Stream};
use realfft::{RealFftPlanner, RealToComplex};

use std::sync::{Arc, Mutex};
use crate::Config;

pub struct AudioData {
    pub spectrum: Vec<f32>,
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
}

fn build_stream<T>(device: &cpal::Device, config: &cpal::StreamConfig, audio_data: Arc<Mutex<AudioData>>, n: usize, r2c: Arc<dyn RealToComplex<f32>>) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: Sample + FromSample<f32> + cpal::SizedSample,
    f32: FromSample<T>,
{
    device.build_input_stream(
        &config,
        move |input_data: &[T], _: &_| {
            let mut shared = audio_data.lock().unwrap();

            let mut out_data = r2c.make_output_vec();
            let mut in_data = vec![0.0; n];
            let len_to_copy = input_data.len().min(n);
            for i in 0..len_to_copy {
                in_data[i] = input_data[i].to_sample::<f32>();
            }
            r2c.process(&mut in_data, &mut out_data).unwrap();

            let spectrum: Vec<f32> = out_data.iter().map(|c| c.norm()).collect();

            let _len = spectrum.len();
            let gain = 0.04;
            shared.bass = (spectrum[0..5].iter().sum::<f32>() / 5.0) * gain;
            shared.mid = (spectrum[5..50].iter().sum::<f32>() / 45.0) * gain * 2.0;
            shared.high = (spectrum[50..200].iter().sum::<f32>() / 150.0) * gain * 0.2;
        },

        |err| eprintln!("Audio error: {}", err),
        None
    )
}

pub fn setup_audio() -> Result<(Arc<Mutex<AudioData>>, Stream, f32), Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string("config.toml").unwrap_or_default();
    let config: Config = toml::from_str(&config_content).unwrap_or(Config { debug_console: false, sensitivity: 1.0 });
    let sensitivity = config.sensitivity;

    let host = cpal::default_host();

    let device = host.default_output_device()
        .expect("Unable to find default output device");

    println!("Using device: {}", device.name()?);

    let config = device.default_input_config()
        .or_else(|_| device.default_output_config())
        .expect("Failed to get audio config");
    let sample_format = config.sample_format();
    let channels = config.channels() as usize;
    let config_inner = config.into();

    let data = Arc::new(Mutex::new(AudioData {
        spectrum: vec![0.0; 512],
        bass: 0.0, mid: 0.0, high: 0.0,
    }));

    let audio_data_clone = Arc::clone(&data);

    let n = 1024;
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n);

    let mut sample_buffer = Vec::with_capacity(n);

    let stream = device.build_input_stream(
        &config_inner,
        move |input_data: &[f32], _: &_| {
            let mut shared = audio_data_clone.lock().unwrap();

            for chunk in input_data.chunks_exact(channels) {
                let mono_sample = chunk.iter().sum::<f32>() / channels as f32;
                sample_buffer.push(mono_sample);

                if sample_buffer.len() == n {
                    let mut out_data = r2c.make_output_vec();
                    r2c.process(&mut sample_buffer, &mut out_data).unwrap();

                    let spectrum: Vec<f32> = out_data.iter().map(|c| c.norm()).collect();

                    shared.spectrum = spectrum.clone();

                    let gain = 0.04 * sensitivity;
                    shared.bass = (spectrum[0..5].iter().sum::<f32>() / 5.0) * gain;
                    shared.mid = (spectrum[5..50].iter().sum::<f32>() / 45.0) * gain * 2.0;
                    shared.high = (spectrum[50..200].iter().sum::<f32>() / 150.0) * gain * 0.2;

                    sample_buffer.clear();
                }
            }
        },

        |err| eprintln!("Audio error: {}", err),
        None
    )?;

    stream.play()?;
    Ok((data, stream, sensitivity))
}

