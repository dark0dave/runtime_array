use serde::{Deserialize, Serialize};

use crate::array::Array;
use std::marker::PhantomData;

#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a, T> {
    marker: PhantomData<&'a T>,
    ptr: *const T,
    end: *const T,
}

impl<'b, T: Serialize + for<'a> Deserialize<'a>> Iter<'b, T> {
    #[inline]
    pub(crate) fn new(array: &'b Array<T>) -> Self {
        let ptr = unsafe { array.get_ptr(0) };
        Self {
            marker: PhantomData,
            ptr,
            end: unsafe { ptr.add(array.size()) },
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr == self.end {
            None
        } else {
            let ptr = self.ptr;
            unsafe {
                self.ptr = self.ptr.add(1);
                Some(&*ptr)
            }
        }
    }
}
