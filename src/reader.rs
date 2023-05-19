#[cfg(feature = "alloc")]
use alloc::{vec, vec::Vec};

use super::{add_error_context, EndianRead, Error, ReadOutput};
use core::mem;
use safe_transmute::{transmute_many_permissive, TriviallyTransmutable};

pub type ReaderResult<T> = Result<T, Error>;

/// An interface to safely read values from a source.
pub trait Reader {
    /// Returns the data to be read from.
    fn get_slice(&self) -> &[u8];

    /// Returns a slice from the given offset.
    /// Returns an empty slice if the offset is greater than the slice size.
    #[inline(always)]
    fn get_slice_at_offset(&self, offset: usize) -> &[u8] {
        let data = self.get_slice();

        if offset >= data.len() {
            return &[];
        }

        &data[offset..]
    }

    /// Gets a slice of bytes from an offset of a source where `slice.len() == size`.
    ///
    /// An error should be returned if the size is invalid (e.g. `offset + size` exceeds the available data)
    /// or if the alignment is incorrect.
    #[inline(always)]
    fn get_slice_of_size(&self, offset: usize, size: usize) -> ReaderResult<&[u8]> {
        let data = self.get_slice();
        let offset_end = offset + size;

        if data.len() < offset_end {
            return Err(Error::InvalidSize {
                wanted_size: size,
                data_len: data.len(),
                offset,
            });
        }

        Ok(&data[offset..offset_end])
    }

    /// Same as [Reader::get_slice_of_size], but uses `T.len()` for the size.
    #[inline(always)]
    fn get_sized_slice<T: Sized>(&self, offset: usize) -> ReaderResult<&[u8]> {
        let data = self.get_slice();
        let result_size = mem::size_of::<T>();
        let offset_end = offset + result_size;

        if data.len() < offset_end {
            return Err(Error::InvalidSize {
                wanted_size: result_size,
                data_len: data.len(),
                offset,
            });
        }

        Ok(&data[offset..offset_end])
    }

    /// Safely gets a [TriviallyTransmutable] reference.
    /// Errors will be returned if the offset does not have enough data for the target type
    /// or is unaligned.
    #[inline(always)]
    fn get_transmutable<T: TriviallyTransmutable>(&self, offset: usize) -> ReaderResult<&T> {
        // Read enough bytes for one of the type
        let bytes = self.get_sized_slice::<T>(offset)?;

        // Transmute to a slice as a hack to transmute a reference
        let read_value =
            transmute_many_permissive::<T>(bytes).map_err(|_| Error::InvalidAlignment {
                wanted_size: mem::size_of::<T>(),
                source_size: bytes.len(),
                source_offset: offset,
            })?;

        // If we get here we're guaranteed to have one value (and only one)
        // so we can unwrap
        Ok(read_value.first().unwrap())
    }

    /// Same as [Reader::get_transmutable], but copies the reference to be an owned value.
    #[inline(always)]
    fn read<T: TriviallyTransmutable>(&self, offset: usize) -> ReaderResult<T> {
        Ok(*self.get_transmutable(offset)?)
    }

    /// Same as [Reader::read], but returns a default value if the read is invalid.
    #[inline(always)]
    fn default_read<T: TriviallyTransmutable + Default>(&self, offset: usize) -> T {
        self.read(offset).unwrap_or_default()
    }

    /// Reads a value from its little endian representation.
    ///
    /// Prefer endian agnostic methods when possible.
    /// This should only be used when reading data from a format or protocol
    /// that explicitly defines little endian.
    #[inline(always)]
    fn read_le_with_output<T: EndianRead>(&self, offset: usize) -> ReaderResult<ReadOutput<T>> {
        let bytes = self.get_slice_at_offset(offset);
        add_error_context(T::try_read_le(bytes), offset, self.get_slice().len())
    }

    /// Same as [Reader::read_le_with_output], but only returns the read data.
    ///
    /// Prefer endian agnostic methods when possible.
    /// This should only be used when reading data from a format or protocol
    /// that explicitly defines little endian.
    #[inline(always)]
    fn read_le<T: EndianRead>(&self, offset: usize) -> ReaderResult<T> {
        let result = self.read_le_with_output(offset)?;
        Ok(result.into_data())
    }

    /// Same as [Reader::read_le], but returns a default value if the read is invalid.
    ///
    /// Prefer endian agnostic methods when possible.
    /// This should only be used when reading data from a format or protocol
    /// that explicitly defines little endian.
    #[inline(always)]
    fn default_read_le<T: EndianRead + Default>(&self, offset: usize) -> T {
        self.read_le(offset).unwrap_or_default()
    }

    /// Reads a value from its big endian representation.
    ///
    /// Prefer endian agnostic methods when possible.
    /// This should only be used when reading data from a format or protocol
    /// that explicitly defines big endian.
    #[inline(always)]
    fn read_be_with_output<T: EndianRead>(&self, offset: usize) -> ReaderResult<ReadOutput<T>> {
        let bytes = self.get_slice_at_offset(offset);
        add_error_context(T::try_read_be(bytes), offset, self.get_slice().len())
    }

    /// Same as [Reader::read_be_with_output], but only returns the read data.
    ///
    /// Prefer endian agnostic methods when possible.
    /// This should only be used when reading data from a format or protocol
    /// that explicitly defines big endian.
    #[inline(always)]
    fn read_be<T: EndianRead>(&self, offset: usize) -> ReaderResult<T> {
        let result = self.read_be_with_output(offset)?;
        Ok(result.into_data())
    }

    /// Same as [Reader::read_be], but returns a default value if the read is invalid.
    ///
    /// Prefer endian agnostic methods when possible.
    /// This should only be used when reading data from a format or protocol
    /// that explicitly defines big endian.
    #[inline(always)]
    fn default_read_be<T: EndianRead + Default>(&self, offset: usize) -> T {
        self.read_be(offset).unwrap_or_default()
    }

    /// Same as [Reader::get_slice_of_size], but converts the result to a vector.
    #[cfg(feature = "alloc")]
    #[inline(always)]
    fn read_byte_vec(&self, offset: usize, size: usize) -> ReaderResult<Vec<u8>> {
        Ok(self.get_slice_of_size(offset, size)?.to_vec())
    }

    /// Same as [Reader::read_byte_vec], but returns a zeroed
    /// out vector of the correct size if the read is invalid.
    #[cfg(feature = "alloc")]
    #[inline(always)]
    fn default_read_byte_vec(&self, offset: usize, size: usize) -> Vec<u8> {
        self.read_byte_vec(offset, size)
            .unwrap_or_else(|_| vec![0; size])
    }

    /// Reads a array from its little endian representation.
    ///
    /// This should only be used when reading data from a format or protocol
    /// that explicitly defines little endian.
    fn read_array_le<const SIZE: usize, T: EndianRead>(
        &self,
        mut offset: usize,
    ) -> ReaderResult<[T; SIZE]> {
        let mut data: [Option<T>; SIZE] = core::array::from_fn(|_| None);

        for elem in &mut data {
            let read_output = self.read_le_with_output::<T>(offset)?;
            offset += read_output.get_read_bytes();
            *elem = Some(read_output.into_data());
        }

        // Safety
        // [T]::map has a hard time optimizing, Option::unwrap_unchecked here can help
        // get rid of panic checks since we know all elements are initialized
        Ok(data.map(|elem| unsafe { elem.unwrap_unchecked() }))
    }

    /// Same as [Reader::read_array_le], but returns a default
    /// array if the read is invalid.
    fn default_read_array_le<const SIZE: usize, T: EndianRead + Default>(
        &self,
        offset: usize,
    ) -> [T; SIZE] {
        // using core::array::from_fn() helps bypass the size limit of default arrays
        self.read_array_le(offset)
            .unwrap_or(core::array::from_fn(|_| T::default()))
    }

    /// Reads a array from its big endian representation.
    ///
    /// This should only be used when reading data from a format or protocol
    /// that explicitly defines big endian.
    fn read_array_be<const SIZE: usize, T: EndianRead>(
        &self,
        mut offset: usize,
    ) -> ReaderResult<[T; SIZE]> {
        let mut data: [Option<T>; SIZE] = core::array::from_fn(|_| None);

        for elem in &mut data {
            let read_output = self.read_be_with_output::<T>(offset)?;
            offset += read_output.get_read_bytes();
            *elem = Some(read_output.into_data());
        }

        // Safety
        // [T]::map has a hard time optimizing, Option::unwrap_unchecked here can help
        // get rid of panic checks since we know all elements are initialized
        Ok(data.map(|elem| unsafe { elem.unwrap_unchecked() }))
    }

    /// Same as [Reader::read_array_be], but returns a default
    /// array if the read is invalid.
    fn default_read_array_be<const SIZE: usize, T: EndianRead + Default>(
        &self,
        offset: usize,
    ) -> [T; SIZE] {
        // using core::array::from_fn() helps bypass the size limit of default arrays
        self.read_array_be(offset)
            .unwrap_or(core::array::from_fn(|_| T::default()))
    }
}

impl<const SIZE: usize> Reader for [u8; SIZE] {
    #[inline(always)]
    fn get_slice(&self) -> &[u8] {
        self
    }
}

impl Reader for &[u8] {
    #[inline(always)]
    fn get_slice(&self) -> &[u8] {
        self
    }
}

impl Reader for &mut [u8] {
    #[inline(always)]
    fn get_slice(&self) -> &[u8] {
        self
    }
}

#[cfg(feature = "alloc")]
impl Reader for Vec<u8> {
    #[inline(always)]
    fn get_slice(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct MockReader {
        bytes: [u8; 8],
    }

    impl MockReader {
        fn new(bytes: [u8; 8]) -> Self {
            Self { bytes }
        }
    }

    impl Reader for MockReader {
        fn get_slice(&self) -> &[u8] {
            &self.bytes
        }
    }

    mod get_slice_of_size {
        use super::*;

        #[test]
        fn should_return_a_slice_of_a_given_size() {
            let reader = MockReader::new([1, 2, 3, 4, 5, 6, 7, 8]);
            let slice = reader
                .get_slice_of_size(4, 4)
                .expect("Read should have been successful.");

            let result = [5, 6, 7, 8];
            assert_eq!(slice, result);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([1, 2, 3, 4, 5, 6, 7, 8]);
            let error = reader
                .get_slice_of_size(6, 4)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }
    }

    mod get_sized_slice {
        use super::*;

        #[test]
        fn should_return_sized_slice() {
            let reader = MockReader::new([1, 2, 3, 4, 5, 6, 7, 8]);
            let slice = reader
                .get_sized_slice::<u32>(4)
                .expect("Read should have been successful.");

            let result = [5, 6, 7, 8];
            assert_eq!(slice, result);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([1, 2, 3, 4, 5, 6, 7, 8]);
            let error = reader
                .get_sized_slice::<u32>(6)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }
    }

    mod get_transmutable {
        use super::*;

        #[test]
        fn should_return_a_reference() {
            let reader = MockReader::new(u64::to_ne_bytes(0x11223344aabbccdd));
            let slice = reader
                .get_sized_slice::<u32>(4)
                .expect("Read should have been successful.");

            let result = u32::to_ne_bytes(0x11223344);
            assert_eq!(slice, &result);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new(u64::to_ne_bytes(0x11223344aabbccdd));
            let error = reader
                .get_sized_slice::<u32>(6)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }
    }

    mod read {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new(u64::to_ne_bytes(0x1122334411223344));
            let value = reader
                .read::<u32>(4)
                .expect("Read should have been successful.");

            assert_eq!(value, 0x11223344);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new(u64::to_ne_bytes(0x1122334411223344));
            let error = reader
                .read::<u32>(6)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }

        #[test]
        fn should_return_error_if_alignment_is_invalid() {
            let reader = MockReader::new(u64::to_ne_bytes(0x1122334411223344));
            let error = reader
                .read::<u32>(3)
                .expect_err("Alignment should have been invalid");

            assert_eq!(
                error,
                Error::InvalidAlignment {
                    wanted_size: 4,
                    source_size: 4,
                    source_offset: 3,
                }
            );
        }
    }

    mod default_read {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new(u64::to_ne_bytes(0x11223344aabbccdd));
            let value = reader.default_read::<u32>(4);
            assert_eq!(value, 0x11223344);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let reader = MockReader::new(u64::to_ne_bytes(0x11223344aabbccdd));
            let value = reader.default_read::<u32>(6);
            assert_eq!(value, u32::default());
        }

        #[test]
        fn should_return_default_if_alignment_is_invalid() {
            let reader = MockReader::new(u64::to_ne_bytes(0x11223344aabbccdd));
            let value = reader.default_read::<u32>(3);
            assert_eq!(value, u32::default());
        }
    }

    mod read_le_with_output {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader
                .read_le_with_output::<u32>(4)
                .expect("Read should have been successful.");

            assert_eq!(value, ReadOutput::new(0xddccbbaa, 4));
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let error = reader
                .read_le_with_output::<u32>(6)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }

        #[derive(Debug)]
        struct CustomErrorTest;

        impl EndianRead for CustomErrorTest {
            fn try_read_le(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                Err(Error::InvalidRead {
                    message: "Custom error!",
                })
            }

            fn try_read_be(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                unimplemented!()
            }
        }

        #[test]
        fn should_bubble_up_custom_errors() {
            let result = vec![].read_le::<CustomErrorTest>(0).unwrap_err();
            let expected = Error::InvalidRead {
                message: "Custom error!",
            };
            assert_eq!(result, expected)
        }

        #[derive(Debug)]
        struct OffsetErrorTest(u32);

        impl EndianRead for OffsetErrorTest {
            fn try_read_le(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                Err(Error::InvalidSize {
                    wanted_size: 8,
                    offset: 1,
                    data_len: 0,
                })
            }

            fn try_read_be(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                unimplemented!()
            }
        }

        #[test]
        fn should_bubble_up_error_offsets() {
            let bytes = vec![];
            let result = bytes.read_le::<OffsetErrorTest>(2).unwrap_err();
            let expected = Error::InvalidSize {
                wanted_size: 8,
                offset: 3,
                data_len: 0,
            };
            assert_eq!(result, expected)
        }
    }

    mod read_le {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader
                .read_le::<u32>(4)
                .expect("Read should have been successful.");

            assert_eq!(value, 0xddccbbaa);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let error = reader
                .read_le::<u32>(6)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }
    }

    mod default_read_le {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader.default_read_le::<u32>(4);
            assert_eq!(value, 0xddccbbaa);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader.default_read_le::<u32>(6);
            assert_eq!(value, u32::default());
        }
    }

    mod read_be_with_output {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader
                .read_be_with_output::<u32>(4)
                .expect("Read should have been successful.");

            assert_eq!(value, ReadOutput::new(0xaabbccdd, 4));
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let error = reader
                .read_be_with_output::<u32>(6)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }

        #[derive(Debug)]
        struct CustomErrorTest;

        impl EndianRead for CustomErrorTest {
            fn try_read_le(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                unimplemented!()
            }

            fn try_read_be(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                Err(Error::InvalidRead {
                    message: "Custom error!",
                })
            }
        }

        #[test]
        fn should_bubble_up_custom_errors() {
            let result = vec![].read_be::<CustomErrorTest>(0).unwrap_err();
            let expected = Error::InvalidRead {
                message: "Custom error!",
            };
            assert_eq!(result, expected)
        }

        #[derive(Debug)]
        struct OffsetErrorTest(u32);

        impl EndianRead for OffsetErrorTest {
            fn try_read_le(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                unimplemented!()
            }

            fn try_read_be(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                Err(Error::InvalidSize {
                    wanted_size: 8,
                    offset: 1,
                    data_len: 0,
                })
            }
        }

        #[test]
        fn should_bubble_up_error_offsets() {
            let bytes = vec![];
            let result = bytes.read_be::<OffsetErrorTest>(2).unwrap_err();
            let expected = Error::InvalidSize {
                wanted_size: 8,
                offset: 3,
                data_len: 0,
            };
            assert_eq!(result, expected)
        }
    }

    mod read_be {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader
                .read_be::<u32>(4)
                .expect("Read should have been successful.");

            assert_eq!(value, 0xaabbccdd);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let error = reader
                .read_be::<u32>(6)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }
    }

    mod default_read_be {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader.default_read_be::<u32>(4);
            assert_eq!(value, 0xaabbccdd);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader.default_read_be::<u32>(6);
            assert_eq!(value, u32::default());
        }
    }

    mod read_byte_vec {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader
                .read_byte_vec(4, 3)
                .expect("Read should have been successful.");

            assert_eq!(value, vec![0xaa, 0xbb, 0xcc]);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let error = reader
                .read_byte_vec(6, 4)
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 6,
                    data_len: 8,
                }
            );
        }
    }

    mod default_read_byte_vec {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader.default_read_byte_vec(4, 3);
            assert_eq!(value, vec![0xaa, 0xbb, 0xcc]);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader.default_read_byte_vec(6, 4);
            assert_eq!(value, vec![0, 0, 0, 0]);
        }
    }

    mod read_array_le {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value: [u16; 4] = reader
                .read_array_le(0)
                .expect("Should have been successful");
            assert_eq!(value, [0x2211, 0x4433, 0xbbaa, 0xddcc]);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader
                .read_array_le::<4, u16>(6)
                .expect_err("Length should have been too large");
            assert_eq!(
                value,
                Error::InvalidSize {
                    wanted_size: 2,
                    offset: 8,
                    data_len: 8,
                }
            );
        }
    }

    mod default_read_array_le {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value: [u16; 4] = reader.default_read_array_le(0);
            assert_eq!(value, [0x2211, 0x4433, 0xbbaa, 0xddcc]);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader.default_read_array_le::<4, u16>(6);

            assert_eq!(value, [0, 0, 0, 0]);
        }
    }

    mod read_array_be {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value: [u16; 4] = reader
                .read_array_be(0)
                .expect("Should have been successful");
            assert_eq!(value, [0x1122, 0x3344, 0xaabb, 0xccdd]);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader
                .read_array_be::<4, u16>(6)
                .expect_err("Length should have been too large");
            assert_eq!(
                value,
                Error::InvalidSize {
                    wanted_size: 2,
                    offset: 8,
                    data_len: 8,
                }
            );
        }
    }

    mod default_read_array_be {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value: [u16; 4] = reader.default_read_array_be(0);
            assert_eq!(value, [0x1122, 0x3344, 0xaabb, 0xccdd]);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let reader = MockReader::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            let value = reader.default_read_array_be::<4, u16>(6);

            assert_eq!(value, [0, 0, 0, 0]);
        }
    }
}
