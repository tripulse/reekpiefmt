#![allow(dead_code)]

use std::io::Write;
use std::marker::PhantomData;
use zstd;

/** A list of specification legal audio sampling rates to be coded in. */
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

trait Sample {
    fn index() -> u8;
    fn to_bytes(&self) -> &'static [u8];
}

macro_rules! sample_impl {
    ($int_type: ty, $index:expr) => {
        impl Sample for $int_type {
            fn index() -> u8 {$index}
            fn to_bytes(&self) -> &'static [u8] {
                let b = self.to_be_bytes();
                unsafe { std::slice::from_raw_parts(b.as_ptr(), b.len()) }
            }
        }
    };
}

sample_impl!(i8,  0);
sample_impl!(i16, 1);
sample_impl!(i32, 2);
sample_impl!(i64, 3);
sample_impl!(f32, 4);
sample_impl!(f64, 5);

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

    fn encode_flat(&mut self, samples: &[S]) -> Option<()> {
        if samples.len() % self.1 != 0 {
            return None;
        }

        self.0.write(
            samples.iter()
            .flat_map(|s| s.to_bytes().iter().map(|s| *s))
            .collect::<Vec<u8>>()
            .as_slice())
        .ok()?;

        Some(())
    }
}

impl<S> Drop for Encoder<S> {
    fn drop(&mut self) {
        self.0.flush().unwrap();
    }
}

#[test]
fn sine_enc() {
    let mut sine = vec![0f32; 44100*4];
    let out = File::create("hello.r2").unwrap();

    {
        for (i, s) in sine.iter_mut().enumerate() {
            *s = (2.0 * PI * i as f32/44100.0).sin();
        }
    }

    let mut r = Encoder::<f32>::new(out, 44100, 1, None)
                .expect("Failed to initialise an RKPI2 instance");

    r.encode_flat(&sine).unwrap();
}