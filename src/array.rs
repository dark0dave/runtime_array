use std::{fmt::Debug, marker::PhantomData, mem::ManuallyDrop, ptr::NonNull};

use serde::{de::Visitor, Deserialize, Serialize};

use crate::iter::Iter;

#[derive(Eq, PartialOrd, Ord)]
#[repr(C, packed)]
pub struct Array<T: Serialize + for<'a> Deserialize<'a> + Copy> {
    pub(crate) pointer: NonNull<T>,
    pub(crate) size: usize,
}

impl<T: ?Sized + Serialize + for<'a> Deserialize<'a> + Copy> Array<T> {
    #[inline]
    pub fn new(size: usize) -> Array<T> {
        Self {
            pointer: NonNull::dangling(),
            size,
        }
    }
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter::new(self)
    }
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.iter().nth(index)
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Copy + Clone> Clone for Array<T> {
    fn clone(&self) -> Self {
        let out = Self::new(self.size);
        unsafe {
            std::ptr::copy_nonoverlapping(self.pointer.as_ptr(), out.pointer.as_ptr(), self.size);
        }
        out
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Copy> Drop for Array<T> {
    fn drop(&mut self) {
        unsafe {
            std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                self.pointer.as_ptr(),
                self.size,
            ))
        }
    }
}

impl<'b, T: Serialize + for<'a> Deserialize<'a> + Copy> IntoIterator for &'b Array<T> {
    type Item = &'b T;
    type IntoIter = Iter<'b, T>;
    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Copy> From<Box<[T]>> for Array<T> {
    #[inline]
    fn from(value: Box<[T]>) -> Self {
        let parts = Box::into_raw(value);
        unsafe {
            Self {
                pointer: NonNull::new_unchecked(parts.cast()),
                size: (*parts).len(),
            }
        }
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Copy> From<Array<T>> for Vec<T> {
    #[inline]
    fn from(vec: Array<T>) -> Self {
        let value = ManuallyDrop::new(vec);
        unsafe { Vec::from_raw_parts(value.pointer.as_ptr(), value.size, value.size) }
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Copy> From<Array<T>> for Box<[T]> {
    #[inline]
    fn from(vec: Array<T>) -> Self {
        let array = ManuallyDrop::new(vec);
        let ptr = std::ptr::slice_from_raw_parts_mut(array.pointer.as_ptr(), array.size);
        unsafe { Box::from_raw(ptr) }
    }
}

impl<'de, T: Serialize + for<'a> Deserialize<'a> + 'de + Copy> Deserialize<'de> for Array<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ArrayVisitor<'de, T> {
            out: PhantomData<&'de T>,
        }

        impl<'de, T: Serialize + for<'a> Deserialize<'a> + Copy> Visitor<'de> for ArrayVisitor<'de, T> {
            type Value = Array<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "struct ArrayVisitor")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                // TODO: Assign in one block
                let mut destination = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Ok(Some(item)) = seq.next_element::<T>() {
                    destination.push(item);
                }
                Ok(Array::from(destination.into_boxed_slice()))
            }
        }

        let visitor = ArrayVisitor { out: PhantomData };
        deserializer.deserialize_seq(visitor)
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Copy> Serialize for Array<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Copy + PartialEq> PartialEq for Array<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }
        if !self.into_iter().eq(other.into_iter()) {
            return false;
        }
        true
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Copy + Debug> Debug for Array<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.into_iter().collect::<Vec<_>>().fmt(f)
    }
}

#[cfg(test)]
mod tests {

    use pretty_assertions::assert_eq;

    use super::*;

    #[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd)]
    #[repr(C, packed)]
    struct Example {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
        e: u128,
        f: i8,
        g: i16,
        h: i32,
        i: i64,
        j: i128,
    }

    #[derive(Deserialize)]
    #[repr(C, packed)]
    struct ArrayExample(Array<Example>);

    impl Serialize for ArrayExample {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.collect_seq(self.0.iter())
        }
    }

    impl PartialEq for ArrayExample {
        fn eq(&self, other: &Self) -> bool {
            self.0.partial_cmp(&other.0).is_some()
        }
    }

    impl Debug for ArrayExample {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("ArrayExample").field(&self.0).finish()
        }
    }

    #[derive(Deserialize)]
    #[repr(C, packed)]
    struct StackArrayExample(ArrayExample);

    impl Serialize for StackArrayExample {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.collect_seq(self.0 .0.iter())
        }
    }

    impl PartialEq for StackArrayExample {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    impl Debug for StackArrayExample {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("StackArrayExample").field(&self.0).finish()
        }
    }

    fn copy_buff_to_struct<T>(buffer: &[u8]) -> T {
        unsafe { std::ptr::read(buffer.as_ptr() as *const _) }
    }

    fn to_u8_slice<T: Sized>(p: &T) -> &[u8] {
        unsafe {
            return core::slice::from_raw_parts(
                (p as *const T) as *const u8,
                std::mem::size_of::<T>(),
            );
        }
    }

    #[test]
    fn array_size() {
        assert_eq!(std::mem::size_of::<ArrayExample>(), 16)
    }

    #[test]
    fn deserialize_json_serialize() {
        let example = Array::from(
            (1..10)
                .map(|i| Example {
                    a: 1_u8,
                    b: i as u16,
                    c: i as u32,
                    d: i as u64,
                    e: i as u128,
                    f: 1_i8,
                    g: i as i16,
                    h: i as i32,
                    i: i as i64,
                    j: i as i128,
                })
                .collect::<Vec<Example>>()
                .into_boxed_slice(),
        );
        let expected = ArrayExample(example);

        let value = serde_json::to_string(&expected).unwrap();

        let result: ArrayExample = serde_json::from_slice(value.as_bytes()).unwrap();
        assert_eq!(
            result.0.get(0),
            Some(&Example {
                a: 1_u8,
                b: 1_u16,
                c: 1_u32,
                d: 1_u64,
                e: 1_u128,
                f: 1_i8,
                g: 1_i16,
                h: 1_i32,
                i: 1_i64,
                j: 1_i128,
            })
        );
        assert_eq!(
            result.0.get(8),
            Some(&Example {
                a: 1_u8,
                b: 9_u16,
                c: 9_u32,
                d: 9_u64,
                e: 9_u128,
                f: 1_i8,
                g: 9_i16,
                h: 9_i32,
                i: 9_i64,
                j: 9_i128,
            })
        );
        assert_eq!(expected.0, result.0)
    }

    #[test]
    fn deserialize_binary_serialize() {
        let example: Array<Example> = Array::from(
            (1..10)
                .map(|i| Example {
                    a: i as u8,
                    b: u16::MAX,
                    c: u32::MAX,
                    d: u64::MAX,
                    e: u128::MAX,
                    f: i as i8,
                    g: i16::MIN,
                    h: i32::MAX,
                    i: i64::MIN,
                    j: i128::MAX,
                })
                .collect::<Vec<Example>>()
                .into_boxed_slice(),
        );
        let expected = StackArrayExample(ArrayExample(example));
        let result: StackArrayExample = copy_buff_to_struct(to_u8_slice(&expected));
        assert_eq!(expected.0 .0, result.0 .0)
    }

    #[test]
    fn deserialize_serialize_ten_times() {
        let example: Array<Example> = Array::from(
            (1..10)
                .map(|i| Example {
                    a: i,
                    b: u16::MAX,
                    c: u32::MAX,
                    d: u64::MAX,
                    e: u128::MAX,
                    f: i as i8,
                    g: i16::MIN,
                    h: i32::MAX,
                    i: i64::MIN,
                    j: i128::MAX,
                })
                .collect::<Vec<Example>>()
                .into_boxed_slice(),
        );
        let expected = StackArrayExample(ArrayExample(example));
        let mut result: StackArrayExample = copy_buff_to_struct(to_u8_slice(&expected));
        for _i in 0..10 {
            let value = serde_json::to_string(&expected).unwrap();
            result = serde_json::from_slice(value.as_bytes()).unwrap();
            result = copy_buff_to_struct(to_u8_slice(&result));
        }
        assert_eq!(
            result.0 .0.get(0),
            Some(&Example {
                a: 1,
                b: u16::MAX,
                c: u32::MAX,
                d: u64::MAX,
                e: u128::MAX,
                f: 1,
                g: i16::MIN,
                h: i32::MAX,
                i: i64::MIN,
                j: i128::MAX,
            })
        );
        assert_eq!(
            result.0 .0.get(8),
            Some(&Example {
                a: 9,
                b: u16::MAX,
                c: u32::MAX,
                d: u64::MAX,
                e: u128::MAX,
                f: 9,
                g: i16::MIN,
                h: i32::MAX,
                i: i64::MIN,
                j: i128::MAX,
            })
        );
        assert_eq!(result, expected)
    }
}
