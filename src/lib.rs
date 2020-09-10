#![allow(dead_code)]

use std::io::Write;
use std::io;
use std::marker::PhantomData;
use zstd;

mod utils;
use utils::*;

#[cfg(test)] mod tests;

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

pub struct Encoder<S> {
    out: Box<dyn Write>,

    num_channels: usize,
    sample_buf: *mut u8,

    _0: PhantomData<S>
}

impl<S> Encoder<S>
    where S: Sample
{
    pub fn new<W: Write + 'static>(
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
            S::INDEX >> 2,                      // sampleformat index.
            (S::INDEX & 3) << 6 |
            (SAMPLERATES                         // samplerate index.
                .iter()
                .position(|&s| s == samplerate)? as u8) << 3 |
            channels-1,                          // number of channels.
        ]).ok()?;

        Some(Self {
            out: match compression {
                Some(l) => Box::new(zstd::Encoder::new(output, l).ok()?),
                None    => Box::new(output)
            },

            num_channels: channels as usize,
            sample_buf: vec![0; S::_SIZE].as_mut_ptr(),
            
            _0: PhantomData
        })
    }

    unsafe fn encode_flat_unchecked(&mut self, samples: &[S]) -> io::Result<usize> {
        self.out.write(
            samples.iter()
                .flat_map(|s| {
                    s.to_bytes(self.sample_buf);

                    (0..S::_SIZE)
                        .map(|i| *self.sample_buf.add(i))
                })
                .collect::<Vec<u8>>()
                .as_slice())
    }

    pub fn encode(&mut self, samples: &[&[S]]) -> Option<()> {
        if samples.len() != self.num_channels {
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

    pub fn encode_flat(&mut self, samples: &[S]) -> Option<()> {
        if samples.len() % self.num_channels != 0 {
            return None;
        }

        self.encode_flat_unchecked(samples).ok()?;
        unsafe {
            self.encode_flat_unchecked(samples)
        }.ok()?;
        Some(())
    }
}

impl<S> Drop for Encoder<S> {
    fn drop(&mut self) {
        self.out.flush().unwrap();
    }
}