use serde::{Deserialize, Serialize};

use crate::iter::Iter;

#[derive(Debug, Copy)]
#[repr(C, packed)]
pub struct Array<T: Serialize + for<'a> Deserialize<'a>> {
    pointer: *mut T,
    size: usize,
}

impl<'de, T: Serialize + for<'a> Deserialize<'a>> Deserialize<'de> for Array<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

impl<T: Serialize + for<'a> Deserialize<'a>> Serialize for Array<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

impl<T: Serialize + for<'a> Deserialize<'a>> Array<T> {
    #[inline]
    pub fn new(size: usize) -> Array<T> {
        let layout =
            std::alloc::Layout::array::<T>(size).expect("Failed to create layout for array");
        unsafe {
            let ptr = std::alloc::alloc(layout) as *mut T;
            if ptr.is_null() {
                panic!("Allocation returned null pointer")
            } else {
                Self { pointer: ptr, size }
            }
        }
    }
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter::new(self)
    }
    #[inline]
    pub unsafe fn from_pointer(pointer: *mut T, size: usize) -> Self {
        Self { pointer, size }
    }
    #[inline]
    pub unsafe fn get_ptr(&self, index: usize) -> *const T {
        self.pointer.add(index)
    }
    #[inline]
    pub unsafe fn get_mut_ptr(&self, index: usize) -> *mut T {
        self.pointer.add(index)
    }
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }
    #[inline]
    pub unsafe fn unchecked_get(&self, index: usize) -> &T {
        &(*(self.pointer.add(index)))
    }
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if self.size <= index {
            None
        } else {
            unsafe { Some(self.unchecked_get(index)) }
        }
    }
    #[inline]
    unsafe fn unchecked_set(&mut self, index: usize, value: T) {
        *(self.pointer.add(index)) = value
    }
}

impl<'b, T: Serialize + for<'a> Deserialize<'a>> IntoIterator for &'b Array<T> {
    type Item = &'b T;
    type IntoIter = Iter<'b, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Serialize + for<'a> Deserialize<'a>> From<Vec<T>> for Array<T> {
    fn from(vec: Vec<T>) -> Self {
        let size = vec.len();
        let mut array = Array::new(size);

        // TODO: Get raw pointer and manually drop vec
        let mut index = 0_usize;
        for item in vec {
            unsafe {
                array.unchecked_set(index, item);
                index += 1;
            }
        }
        array
    }
}

impl<T: Clone + Serialize + for<'a> Deserialize<'a>> Clone for Array<T> {
    fn clone(&self) -> Self {
        let arr = Array::new(self.size);
        //TODO: Copy entire block not item by item
        for i in 0..self.size {
            unsafe {
                *arr.get_mut_ptr(i) = std::ptr::read(self.get_ptr(i));
            }
        }
        arr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_size() {
        #[derive(Debug, Clone, Copy, Deserialize, Serialize)]
        #[repr(C, packed)]
        struct Example {
            temp: u16,
        }

        #[derive(Debug, Clone, Copy, Deserialize, Serialize)]
        #[repr(C, packed)]
        struct ArrayExample {
            temp: Array<Example>,
        }
        assert_eq!(std::mem::size_of::<ArrayExample>(), 16)
    }
}
