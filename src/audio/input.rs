use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;

use cpal::{
    SampleFormat,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::audio::types::AudioFrame;

pub fn start_audio_input(tx: mpsc::Sender<AudioFrame>) -> Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("No input device");

    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let tx = Arc::new(tx);

    let stream = match config.sample_format() {
        SampleFormat::F32 => build_f32_stream(&device, &config.into(), tx, sample_rate, channels)?,
        SampleFormat::I16 => build_i16_stream(&device, &config.into(), tx, sample_rate, channels)?,
        SampleFormat::U16 => build_u16_stream(&device, &config.into(), tx, sample_rate, channels)?,
        _ => unreachable!(),
    };

    stream.play()?;
    std::mem::forget(stream);

    Ok(())
}

fn build_f32_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: Arc<mpsc::Sender<AudioFrame>>,
    sample_rate: u32,
    channels: usize,
) -> Result<cpal::Stream> {
    let stream = device.build_input_stream(
        config,
        move |data: &[f32], _| {
            send_frames(data, channels, sample_rate, &tx);
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn build_i16_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: Arc<mpsc::Sender<AudioFrame>>,
    sample_rate: u32,
    channels: usize,
) -> Result<cpal::Stream> {
    let stream = device.build_input_stream(
        config,
        move |data: &[i16], _| {
            let data: Vec<f32> = data.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
            send_frames(&data, channels, sample_rate, &tx);
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn build_u16_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: Arc<mpsc::Sender<AudioFrame>>,
    sample_rate: u32,
    channels: usize,
) -> Result<cpal::Stream> {
    let stream = device.build_input_stream(
        config,
        move |data: &[u16], _| {
            let data: Vec<f32> = data
                .iter()
                .map(|s| (*s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                .collect();
            send_frames(&data, channels, sample_rate, &tx);
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn send_frames(data: &[f32], channels: usize, sample_rate: u32, tx: &mpsc::Sender<AudioFrame>) {
    let mut samples = Vec::with_capacity(data.len() / channels);
    for frame in data.chunks(channels) {
        samples.push(frame[0]);
    }

    let _ = tx.try_send(AudioFrame {
        samples,
        sample_rate,
    });
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("audio input error: {err}");
}
