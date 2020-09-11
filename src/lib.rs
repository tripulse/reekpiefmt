#![allow(dead_code)]

use std::io::{Read, Write};
use std::io;
use std::convert::TryInto;
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
    byte_sample_buf: Vec<u8>,
    flat_sample_buf: Vec<S>
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
            S::INDEX >> 2,                       // sampleformat index.
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

            byte_sample_buf: vec![],
            flat_sample_buf: vec![]
        })
    }

    unsafe fn encode_flat_unchecked(&mut self) -> io::Result<usize> {
        self.byte_sample_buf.resize(self.flat_sample_buf.len() * S::_SIZE, 0);

        for (i, s) in self.flat_sample_buf.iter().enumerate() {
            let bi = i * S::_SIZE;  // byte index in byte_sample_buf.
            s.to_bytes(&mut self.byte_sample_buf[bi..bi+S::_SIZE]);
        }

        self.out.write(&self.byte_sample_buf)
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

        // fill by the first channel's first element, since Sample isn't constructible.
        self.flat_sample_buf.resize(min_samples * samples.len(), samples[0][0]);
        
        for s in 0..min_samples {
            for c in 0..self.num_channels {
                self.flat_sample_buf[s*self.num_channels + c] = samples[c][s];  
            }
        }

        unsafe { self.encode_flat_unchecked() }.ok()?;
        Some(())
    }

    pub fn encode_flat(&mut self, samples: &[S]) -> Option<()> {
        if samples.len() % self.num_channels != 0 {
            return None;
        }

        unsafe {
            self.flat_sample_buf.copy_from_slice(samples);
            self.encode_flat_unchecked()
        }.ok()?;
        Some(())
    }
}

impl<S> Drop for Encoder<S> {
    fn drop(&mut self) {
        self.out.flush().unwrap();
    }
}

#[derive(Debug, Clone, PartialEq)]
enum DynamicSampleBuf {
    Int8   (Vec<i8>),
    Int16  (Vec<i16>),
    Int32  (Vec<i32>),
    Int64  (Vec<i64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SampleFormat {
    Int8    = 0,
    Int16   = 1,
    Int32   = 2,
    Int64   = 3,
    Float32 = 4,
    Float64 = 5
}

struct Decoder {
    input: Box<dyn Read>,
    sample_fmt: SampleFormat,
    sample_rate: u32,
    num_channels: usize,

    block_size: usize  // size of each sample block.
}

impl Decoder {
    fn new<R>(mut input: R) -> Option<Self> 
        where R: Read + 'static
    {
        let mut hdr = [0u8; 2];
        input.read_exact(&mut hdr[..]).ok()?;

        if hdr[0] >> 2 != 61 {
            return None;
        }

        let sample_fmt = unsafe {
            std::mem::transmute::<_, _>(
                (hdr[0] & 1) << 2 | hdr[1] >> 6
        )};


        Some(Decoder {
            input: match (hdr[0] & 3) >> 1 {
                0 => Box::new(input),
                1 => Box::new(zstd::Decoder::new(input).ok()?),
                _ => return None  // rust is forcing here ;P
            },
            sample_fmt,
            sample_rate: SAMPLERATES[(hdr[1] >> 3 & 7) as usize],
            num_channels: ((hdr[1] & 7) + 1) as usize,

            block_size: match sample_fmt {
                SampleFormat::Int8    => 1,
                SampleFormat::Int16   => 2,
                SampleFormat::Int32   => 4,
                SampleFormat::Int64   => 8,
                SampleFormat::Float32 => 4,
                SampleFormat::Float64 => 8
            }
        })
    }

    fn sample_format(&self) -> SampleFormat { self.sample_fmt         }
    fn sample_rate(&self)   -> u32          { self.sample_rate        }
    fn num_channels(&self)  -> u8           { self.num_channels as u8 }

    fn decode_flat(&mut self, n: usize) -> Option<DynamicSampleBuf> {
        let mut buf = vec![0u8; n * self.block_size * self.num_channels];
        let bufsiz = self.input.read(&mut buf).ok()?;
        
        // misaligned buffer is entirely discarded, in future
        // in future it will be cropped to its largest possible
        // aligned size.
        if bufsiz % self.block_size                      != 0 ||
          (bufsiz / self.block_size) % self.num_channels != 0 {
            return None;
        }

        buf.truncate(bufsiz);  // truncate this, to prevent invalid reads.

        macro_rules! parse_nums {
            ($num: ty) => {
                buf
                .chunks(self.block_size)
                .map(|x| <$num>::from_be_bytes(x.try_into().unwrap()))
                .collect::<Vec<$num>>()
            };
        }

        Some(match self.sample_fmt {
            SampleFormat::Int8    => DynamicSampleBuf::Int8   (parse_nums!(i8)),
            SampleFormat::Int16   => DynamicSampleBuf::Int16  (parse_nums!(i16)),
            SampleFormat::Int32   => DynamicSampleBuf::Int32  (parse_nums!(i32)),
            SampleFormat::Int64   => DynamicSampleBuf::Int64  (parse_nums!(i64)),
            SampleFormat::Float32 => DynamicSampleBuf::Float32(parse_nums!(f32)),
            SampleFormat::Float64 => DynamicSampleBuf::Float64(parse_nums!(f64))
        })
    }
}