use std::iter;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct Fifo<T>(Vec<T>);

impl<T> Fifo<T> {
    pub fn new(sz: usize) -> Fifo<T> where T: Default + Clone {
        let mut buffer = Vec::with_capacity(sz);
        buffer.extend(iter::repeat(Default::default()).take(sz));
        Fifo(buffer)
    }

    pub fn push(&mut self, data: &[T]) where T: Copy {
        let dlen = data.len();
        let blen = self.0.len();

        if dlen >= blen {
            self.0.copy_from_slice(&data[dlen - blen ..]);
        } else {
            self.0.copy_within(dlen .., 0);
            (&mut self.0[blen - dlen ..]).copy_from_slice(data);
        }
    }

    pub fn size(&self) -> usize { self.0.len() }

    pub fn resize(&mut self, newsz: usize) where T: Default + Clone {
        self.0.resize(newsz, Default::default())
    }
}

impl<T> Deref for Fifo<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { &self.0 }
}

impl<T> DerefMut for Fifo<T> {
    fn deref_mut(&mut self) -> &mut [T] { &mut self.0 }
}
