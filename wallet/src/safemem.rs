use heapless::{ArrayLength, Vec};

pub struct SafeVec<B>(Vec<u8, B>)
where
    B: ArrayLength<u8>;

impl<B> Drop for SafeVec<B>
where
    B: ArrayLength<u8>,
{
    fn drop(&mut self) {
        self.0.iter_mut().for_each(|b| *b = 0xFF)
    }
}
