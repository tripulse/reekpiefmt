pub trait Sample: Copy {
    const _SIZE: usize;
    const INDEX: u8;

    unsafe fn to_bytes(&self, out: *mut u8);
}

macro_rules! sample_impl {
    ($int_type: ty, $index:expr) => {
        impl Sample for $int_type {
            const _SIZE: usize = std::mem::size_of::<$int_type>();
            const INDEX: u8 = $index;

            unsafe fn to_bytes(&self, out: *mut u8) {
                std::ptr::copy(self.to_be_bytes().as_ptr(), out, Self::_SIZE);
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