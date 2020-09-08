pub trait Sample: Copy {
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