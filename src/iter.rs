use crate::array::Array;

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a, T> {
    marker: PhantomData<&'a T>,
    ptr: *const T,
    end: *const T,
    done: bool,
}

impl<'b, T: Serialize + for<'a> Deserialize<'a> + Copy> Iter<'b, T> {
    #[inline]
    pub(crate) fn new(array: &'b Array<T>) -> Self {
        let ptr = array.pointer;
        Self {
            marker: PhantomData,
            ptr,
            end: unsafe { ptr.add(array.size - 1) },
            done: false,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if self.ptr == self.end {
            self.done = true
        }
        let ptr = self.ptr;
        unsafe {
            self.ptr = self.ptr.add(1);
            Some(&*ptr)
        }
    }
}
