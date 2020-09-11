pub trait Sample: Copy {
    const _SIZE: usize;
    const INDEX: u8;

    unsafe fn to_bytes(&self, out: &mut [u8]) -> Option<()>;
}

macro_rules! sample_impl {
    ($int_type: ty, $index:expr) => {
        impl Sample for $int_type {
            const _SIZE: usize = std::mem::size_of::<$int_type>();
            const INDEX: u8 = $index;

            unsafe fn to_bytes(&self, out: &mut [u8]) -> Option<()> {
                if(out.len() == Self::_SIZE) {
                    std::ptr::copy(self.to_be_bytes().as_ptr(),
                                   out.as_mut_ptr(),
                                   Self::_SIZE);
                    Some(())
                } else { None }
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

#[derive(Debug, Clone, PartialEq)]
pub enum DynamicSampleBuf {
    Int8   (Vec<i8>),
    Int16  (Vec<i16>),
    Int32  (Vec<i32>),
    Int64  (Vec<i64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    Int8    = 0,
    Int16   = 1,
    Int32   = 2,
    Int64   = 3,
    Float32 = 4,
    Float64 = 5
}
