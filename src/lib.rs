#![allow(dead_code)]

use std::io::Write;
use std::io;
use std::marker::PhantomData;
use zstd;

mod utils;
use utils::*;

const SAMPLERATES: [u32; 8] = [
    8000,
    12000,
    22050,
    32000,
    44100,
    64000,
    96000,
    192000
];

struct Encoder<S>(Box<dyn Write>, usize, PhantomData<S>);

impl<S> Encoder<S>
    where S: Sample
{
    fn new<W: Write + 'static>(
        mut output: W,
        samplerate: u32,
        channels: u8,
        compression: Option<i32>
    ) -> Option<Self>
    {
        if channels < 1 || channels > 8 { return None; }
       
        output.write(&[
            244 |                                // identifier.
            (compression.is_some() as u8) << 1 | // if compression applied.
            S::index() >> 2,                     // sampleformat index.
            (S::index() & 3) << 6 |
            (SAMPLERATES                         // samplerate index.
                .iter()
                .position(|&s| s == samplerate)? as u8) << 3 |
            channels-1,                          // number of channels.
        ]).ok()?;

        Some(Self(
            match compression {
                Some(l) => Box::new(zstd::Encoder::new(output, l).ok()?),
                None    => Box::new(output)
            },
            channels as usize, PhantomData))
    }

    fn encode_flat_unchecked(&mut self, samples: &[S]) -> io::Result<usize> {
        self.0.write(
            samples.iter()
                .flat_map(|s| s.to_bytes().iter().map(|s| *s))
                .collect::<Vec<u8>>()
                .as_slice())
    }

    fn encode(&mut self, samples: &[&[S]]) -> Option<()> {
        if samples.len() != self.1 {
            return None;
        }

        let min_samples =
            match samples.iter()
                .min_by(|a, b| a.len().cmp(&b.len()))?
                .len() {
                    0 => return None,
                    l => l
                };

        self.encode_flat_unchecked(
            samples.iter()
                .flat_map(|b| b[..min_samples].iter().map(|s| *s))
                .collect::<Vec<S>>()
                .as_slice()
        ).ok()?;
        
        Some(())
    }

    fn encode_flat(&mut self, samples: &[S]) -> Option<()> {
        if samples.len() % self.1 != 0 {
            return None;
        }

        self.encode_flat_unchecked(samples).ok()?;
        Some(())
    }
}

impl<S> Drop for Encoder<S> {
    fn drop(&mut self) {
        self.0.flush().unwrap();
    }
}