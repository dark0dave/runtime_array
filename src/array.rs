use std::{fmt::Debug, marker::PhantomData, mem::ManuallyDrop, ops::Deref, slice};

use serde::{de::Visitor, Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C, packed)]
pub struct Array<'a, T: Serialize + Deserialize<'a> + Copy + Deref + PartialOrd> {
    pub(crate) pointer: &'a [T],
    size: usize,
}

impl<'a, T: Serialize + Deserialize<'a> + Copy + Deref + PartialOrd> Array<'a, T> {
    #[inline]
    pub fn new(data: &'a [T]) -> Array<'a, T> {
        Self {
            pointer: data,
            size: data.len(),
        }
    }
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.pointer.get(index)
    }
}

impl<'a, T: Serialize + Deserialize<'a> + Copy + PartialEq + PartialOrd + Deref + PartialOrd>
    From<Vec<T>> for Array<'a, T>
{
    fn from(vec: Vec<T>) -> Self {
        let size = vec.len();
        let v: Vec<T> = Vec::with_capacity(size);
        let mut v = ManuallyDrop::new(v);
        let ptr: *mut T = v.as_mut_ptr();

        unsafe {
            let (_head, aligned, _tail) = vec.align_to::<T>();
            for (counter, t) in aligned.iter().enumerate() {
                let tmp = std::ptr::read(t);
                std::ptr::write(ptr.add(counter), tmp);
            }
            let data = slice::from_raw_parts(ptr, size);
            Array::new(data)
        }
    }
}

impl<
        'a,
        T: Serialize + Deserialize<'a> + 'a + Copy + PartialEq + PartialOrd + Deref + PartialOrd,
    > Deserialize<'a> for Array<'a, T>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        struct ArrayVisitor<'a, 'de: 'a, T> {
            outer: PhantomData<&'de T>,
            inner: PhantomData<&'a T>,
        }

        impl<'a: 'de, 'de, T: Serialize + Deserialize<'de> + Copy + Deref + PartialOrd> Visitor<'de>
            for ArrayVisitor<'a, 'de, T>
        {
            type Value = Array<'a, T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "struct VarriableCharArray")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                // TODO: Assign in one block
                let mut destination = vec![];
                while let Ok(Some(item)) = seq.next_element::<T>() {
                    destination.push(item);
                }
                Ok(Array::from(destination))
            }
        }

        let visitor = ArrayVisitor {
            outer: PhantomData,
            inner: PhantomData,
        };
        deserializer.deserialize_seq(visitor)
    }
}

impl<
        'a,
        'de: 'a,
        T: Serialize + Deserialize<'a> + Copy + PartialEq + PartialOrd + Deref + PartialOrd,
    > Serialize for Array<'a, T>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.pointer.iter())
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

    impl Deref for Example {
        type Target = Example;

        fn deref(&self) -> &Self::Target {
            self
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, PartialEq)]
    #[repr(C, packed)]
    struct ArrayExample<'a>(Array<'a, Example>);

    impl<'a, 'de: 'a> Deserialize<'de> for ArrayExample<'a> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(ArrayExample(Array::deserialize::<_>(deserializer).unwrap()))
        }
    }

    #[derive(Debug, Clone, Serialize, PartialEq)]
    #[repr(C, packed)]
    struct StackArrayExample<'a>(ArrayExample<'a>);

    impl<'a, 'de: 'a> Deserialize<'de> for StackArrayExample<'a> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(StackArrayExample(
                ArrayExample::deserialize::<_>(deserializer).unwrap(),
            ))
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
        assert_eq!(std::mem::size_of::<ArrayExample>(), 24)
    }

    #[test]
    fn deserialize_json_serialize() {
        let example = Array::from(
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
                .collect::<Vec<Example>>(),
        );
        let expected = ArrayExample(example);
        assert_eq!(
            expected.0.get(0),
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
        let value = serde_json::to_string(&expected).unwrap();

        let result: ArrayExample = serde_json::from_slice(value.as_bytes()).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_binary_serialize() {
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
                .collect::<Vec<Example>>(),
        );
        let expected = StackArrayExample(ArrayExample(example));
        let result: StackArrayExample = copy_buff_to_struct(to_u8_slice(&expected));
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_serialize_deserialize_serialize() {
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
                .collect::<Vec<Example>>(),
        );
        let expected = StackArrayExample(ArrayExample(example));
        let mut result: StackArrayExample = copy_buff_to_struct(to_u8_slice(&expected));
        let value = serde_json::to_string(&result).unwrap();
        result = serde_json::from_slice(value.as_bytes()).unwrap();
        result = copy_buff_to_struct(to_u8_slice(&result));
        assert_eq!(
            expected.0 .0.get(0),
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
        assert_eq!(result, expected);
    }
}
