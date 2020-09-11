extern crate periodicsynth;

use periodicsynth::{synth, sin};

use std::fs::File;
use super::*;


#[test]
fn sine_enc() {
    let mut r = Encoder::new(
        File::create("sample.r2").unwrap(),
        44100, // 44.1kHz samplerate
        2,     // 2       channels
        None   // no      compression
    ).unwrap();

    // the length will be clipped to 1/2 seconds.
    // in future: there might be an option to pad samplebuffers.
    r.encode(&vec![
        &synth(sin, &mut 800f64, 44100)[..],  // L: 800hZ
        &synth(sin, &mut 600f64, 22050)[..]   // R: 600hZ
    ][..]).unwrap();
}

#[test]
fn sine_dec() {
    let r = Decoder::new(
        File::open("sample.r2").unwrap()
    ).unwrap();

    assert_eq!(r.sample_rate(), 44100);
    assert_eq!(r.num_channels(), 2);
    assert_eq!(r.sample_format(), SampleFormat::Float64);
}