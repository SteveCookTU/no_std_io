use super::{
    cursor::Cursor,
    iter::{BeIter, LeIter},
};
use crate::{EndianRead, Reader, ReaderResult};
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use safe_transmute::TriviallyTransmutable;

/// An interface to read values as a stream.
pub trait StreamReader: Reader + Cursor + Sized {
    /// Same as [Reader::read], but uses the current stream instead of an offset.
    #[inline(always)]
    fn read_stream<T: TriviallyTransmutable + Default>(&mut self) -> ReaderResult<T> {
        let index = self.swap_incremented_index_for_type::<T>();
        self.read(index)
    }

    /// Same as [StreamReader::read_stream], but returns a default value if the read is invalid.
    #[inline(always)]
    fn default_read_stream<T: TriviallyTransmutable + Default>(&mut self) -> T {
        let index = self.swap_incremented_index_for_type::<T>();
        self.default_read(index)
    }

    /// Same as [Reader::read_le], but uses the current stream instead of an offset.
    #[inline(always)]
    fn read_stream_le<T: EndianRead>(&mut self) -> ReaderResult<T> {
        let index = self.get_index();
        let read_value = self.read_le_with_output(index)?;
        self.increment_by(read_value.get_read_bytes());
        Ok(read_value.into_data())
    }

    /// Same as [StreamReader::read_stream_le], but returns a default value if the read is invalid.
    #[inline(always)]
    fn default_read_stream_le<T: EndianRead + Default>(&mut self) -> T {
        let index = self.swap_incremented_index_for_type::<T>();
        self.default_read_le(index)
    }

    /// Same as [Reader::read_array_le], but uses the current stream instead of an offset.
    ///
    /// The index is only incremented on a successful read.
    #[inline(always)]
    fn read_array_stream_le<const SIZE: usize, T: EndianRead>(
        &mut self,
    ) -> ReaderResult<[T; SIZE]> {
        let index = self.get_index();
        let mut read_bytes = 0;

        let mut data: [Option<T>; SIZE] = core::array::from_fn(|_| None);

        for elem in &mut data {
            let read_output = self.read_le_with_output::<T>(index + read_bytes)?;
            read_bytes += read_output.get_read_bytes();
            *elem = Some(read_output.into_data());
        }

        self.increment_by(read_bytes);

        // Safety
        // [T]::map has a hard time optimizing, Option::unwrap_unchecked here can help
        // get rid of panic checks since we know all elements are initialized
        Ok(data.map(|elem| unsafe { elem.unwrap_unchecked() }))
    }

    /// Same as [StreamReader::read_stream_le], but returns a default array if the read is invalid.
    ///
    /// The returned value contains everything that could be read with the rest being the default.
    #[inline(always)]
    fn default_read_array_stream_le<const SIZE: usize, T: EndianRead + Default>(
        &mut self,
    ) -> [T; SIZE] {
        self.read_array_stream_le()
            .unwrap_or(core::array::from_fn(|_| T::default()))
    }

    /// Same as [Reader::read_be], but uses the current stream instead of an offset.
    #[inline(always)]
    fn read_stream_be<T: EndianRead>(&mut self) -> ReaderResult<T> {
        let index = self.get_index();
        let read_value = self.read_be_with_output(index)?;
        self.increment_by(read_value.get_read_bytes());
        Ok(read_value.into_data())
    }

    /// Same as [StreamReader::read_stream_be], but returns a default value if the read is invalid.
    #[inline(always)]
    fn default_read_stream_be<T: EndianRead + Default>(&mut self) -> T {
        let index = self.swap_incremented_index_for_type::<T>();
        self.default_read_be(index)
    }

    /// Same as [Reader::read_array_be], but uses the current stream instead of an offset.
    ///
    /// The index is only incremented on a successful read.
    #[inline(always)]
    fn read_array_stream_be<const SIZE: usize, T: EndianRead>(
        &mut self,
    ) -> ReaderResult<[T; SIZE]> {
        let index = self.get_index();
        let mut read_bytes = 0;

        let mut data: [Option<T>; SIZE] = core::array::from_fn(|_| None);

        for elem in &mut data {
            let read_output = self.read_be_with_output::<T>(index + read_bytes)?;
            read_bytes += read_output.get_read_bytes();
            *elem = Some(read_output.into_data());
        }

        self.increment_by(read_bytes);

        // Safety
        // [T]::map has a hard time optimizing, Option::unwrap_unchecked here can help
        // get rid of panic checks since we know all elements are initialized
        Ok(data.map(|elem| unsafe { elem.unwrap_unchecked() }))
    }

    /// Same as [StreamReader::read_stream_be], but returns a default array if the read is invalid.
    #[inline(always)]
    fn default_read_array_stream_be<const SIZE: usize, T: EndianRead + Default>(
        &mut self,
    ) -> [T; SIZE] {
        self.read_array_stream_be()
            .unwrap_or(core::array::from_fn(|_| T::default()))
    }

    /// Same as [Reader::read_byte_vec], but uses the current stream instead of an offset.
    #[cfg(feature = "alloc")]
    #[inline(always)]
    fn read_byte_stream(&mut self, size: usize) -> ReaderResult<Vec<u8>> {
        let index = self.swap_incremented_index(size);
        self.read_byte_vec(index, size)
    }

    /// Same as [Reader::default_read_byte_vec], but returns a default value if the read is invalid.
    #[cfg(feature = "alloc")]
    #[inline(always)]
    fn default_read_byte_stream(&mut self, size: usize) -> Vec<u8> {
        let index = self.swap_incremented_index(size);
        self.default_read_byte_vec(index, size)
    }

    #[inline(always)]
    fn into_le_iter<Item: EndianRead>(self) -> LeIter<Item, Self> {
        LeIter::new(self)
    }

    #[inline(always)]
    fn into_be_iter<Item: EndianRead>(self) -> BeIter<Item, Self> {
        BeIter::new(self)
    }
}

impl<T> StreamReader for T where T: Reader + Cursor {}

#[cfg(test)]
mod test {
    use super::*;

    pub struct MockStream {
        bytes: [u8; 8],
        index: usize,
    }

    impl MockStream {
        fn new(bytes: [u8; 8]) -> Self {
            Self { bytes, index: 0 }
        }
    }

    impl Reader for MockStream {
        fn get_slice(&self) -> &[u8] {
            &self.bytes
        }
    }

    impl Cursor for MockStream {
        fn get_index(&self) -> usize {
            self.index
        }

        fn set_index(&mut self, index: usize) {
            self.index = index;
        }
    }

    mod read_stream {
        use super::*;
        use crate::Error;

        #[test]
        fn should_return_a_value() {
            let mut reader = MockStream::new(u64::to_ne_bytes(0x1122334411223344));
            let value = reader
                .read_stream::<u32>()
                .expect("Read should have been successful.");

            assert_eq!(value, 0x11223344);
            assert_eq!(reader.get_index(), 4);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let mut reader = MockStream::new(u64::to_ne_bytes(0x1122334411223344));
            reader.set_index(8);
            let error = reader
                .read_stream::<u32>()
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 8,
                    data_len: 8,
                }
            );
        }

        #[test]
        fn should_return_error_if_alignment_is_invalid() {
            let mut reader = MockStream::new(u64::to_ne_bytes(0x1122334411223344));
            reader.set_index(3);
            let error = reader
                .read_stream::<u32>()
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

    mod default_read_stream {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let mut reader = MockStream::new(u64::to_ne_bytes(0x11223344aabbccdd));
            reader.set_index(4);
            let value = reader.default_read_stream::<u32>();
            assert_eq!(value, 0x11223344);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let mut reader = MockStream::new(u64::to_ne_bytes(0x11223344aabbccdd));
            reader.set_index(8);
            let value = reader.default_read_stream::<u32>();
            assert_eq!(value, u32::default());
        }

        #[test]
        fn should_return_default_if_alignment_is_invalid() {
            let mut reader = MockStream::new(u64::to_ne_bytes(0x11223344aabbccdd));
            reader.set_index(2);
            let value = reader.default_read_stream::<u32>();
            assert_eq!(value, u32::default());
        }
    }

    mod read_stream_le {
        use super::*;
        use crate::{Error, ReadOutput};

        #[test]
        fn should_return_a_value() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(4);
            let value = reader
                .read_stream_le::<u32>()
                .expect("Read should have been successful.");

            assert_eq!(value, 0xddccbbaa);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(8);
            let error = reader
                .read_stream_le::<u32>()
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 4,
                    offset: 8,
                    data_len: 8,
                }
            );
        }

        #[derive(Debug, PartialEq)]
        struct Sum(u8);

        impl EndianRead for Sum {
            fn try_read_le(bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                let sum = bytes[0].wrapping_add(bytes[1]);
                Ok(ReadOutput::new(Sum(sum), 2))
            }

            fn try_read_be(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                unimplemented!()
            }
        }

        #[test]
        fn should_read_values_with_dynamic_read_lengths() {
            let mut reader = MockStream::new([0x11, 0x22, 0xaa, 0xbb, 0x88, 0x99, 0x01, 0x02]);
            let value = reader
                .read_stream_le::<Sum>()
                .expect("Read should have been successful.");

            assert_eq!(value, Sum(0x33));
        }

        #[test]
        fn should_read_multiple_values_with_dynamic_read_lengths() {
            let mut reader = MockStream::new([0x11, 0x22, 0xaa, 0xbb, 0x88, 0x99, 0x01, 0x02]);

            let value = reader
                .read_stream_le::<Sum>()
                .expect("Read should have been successful.");
            assert_eq!(value, Sum(0x33));

            let value = reader
                .read_stream_le::<Sum>()
                .expect("Read should have been successful.");
            assert_eq!(value, Sum(0x65));
        }
    }

    mod read_array_stream_le {
        use super::*;
        use crate::{Error, ReadOutput};

        #[test]
        fn should_return_a_value() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(4);
            let value = reader
                .read_array_stream_le::<2, u16>()
                .expect("Read should have been successful.");

            assert_eq!(value, [0xbbaa, 0xddcc]);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(8);
            let error = reader
                .read_array_stream_le::<2, u16>()
                .expect_err("Length should have been too large");

            assert_eq!(
                error,
                Error::InvalidSize {
                    wanted_size: 2,
                    offset: 8,
                    data_len: 8,
                }
            );
        }

        #[derive(Debug, PartialEq)]
        struct Sum(u8);

        impl EndianRead for Sum {
            fn try_read_le(bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                let sum = bytes[0].wrapping_add(bytes[1]);
                Ok(ReadOutput::new(Sum(sum), 2))
            }

            fn try_read_be(_bytes: &[u8]) -> Result<ReadOutput<Self>, Error> {
                unimplemented!()
            }
        }

        #[test]
        fn should_read_values_with_dynamic_read_lengths() {
            let mut reader = MockStream::new([0x11, 0x22, 0xaa, 0xbb, 0x88, 0x99, 0x01, 0x02]);
            let value = reader
                .read_array_stream_le::<2, Sum>()
                .expect("Read should have been successful.");

            assert_eq!(value, [Sum(0x33), Sum(0x65)]);
        }

        #[test]
        fn should_read_multiple_values_with_dynamic_read_lengths() {
            let mut reader = MockStream::new([0x11, 0x22, 0xaa, 0xbb, 0x88, 0x99, 0x01, 0x02]);

            let value = reader
                .read_array_stream_le::<2, Sum>()
                .expect("Read should have been successful.");
            assert_eq!(value, [Sum(0x33), Sum(0x65)]);

            let value = reader
                .read_array_stream_le::<2, Sum>()
                .expect("Read should have been successful.");
            assert_eq!(value, [Sum(0x21), Sum(0x3)]);
        }
    }

    mod default_read_array_stream_le {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(4);
            let value = reader.default_read_array_stream_le::<2, u16>();

            assert_eq!(value, [0xbbaa, 0xddcc]);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(6);
            let value = reader.default_read_array_stream_le::<2, u16>();
            assert_eq!(value, [0u16; 2]);
        }
    }

    mod default_read_stream_le {
        use super::*;

        #[test]
        fn should_return_a_value() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(4);
            let value = reader.default_read_stream_le::<u32>();
            assert_eq!(value, 0xddccbbaa);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(6);
            let value = reader.default_read_stream_le::<u32>();
            assert_eq!(value, u32::default());
        }
    }

    mod read_byte_stream {
        use super::*;
        use crate::Error;
        use alloc::vec;

        #[test]
        fn should_return_a_value() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(4);
            let value = reader
                .read_byte_stream(3)
                .expect("Read should have been successful.");

            assert_eq!(value, vec![0xaa, 0xbb, 0xcc]);
        }

        #[test]
        fn should_return_error_if_size_is_too_large_for_offset() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(6);
            let error = reader
                .read_byte_stream(4)
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

    mod default_read_byte_stream {
        use super::*;
        use alloc::vec;

        #[test]
        fn should_return_a_value() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(4);
            let value = reader.default_read_byte_stream(3);
            assert_eq!(value, vec![0xaa, 0xbb, 0xcc]);
        }

        #[test]
        fn should_return_default_if_size_is_too_large_for_offset() {
            let mut reader = MockStream::new([0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]);
            reader.set_index(6);
            let value = reader.default_read_byte_stream(4);
            assert_eq!(value, vec![0, 0, 0, 0]);
        }
    }

    mod into_le_iter {
        use super::*;

        #[test]
        fn should_iterate() {
            let data: [u8; 8] = [0xaa, 0xbb, 0xcc, 0xdd, 0x11, 0x22, 0x33, 0x44];
            let result = MockStream::new(data).into_le_iter().collect::<Vec<u32>>();
            assert_eq!(result, [0xddccbbaa, 0x44332211]);
        }

        #[test]
        fn should_iterate_from_cursor() {
            let data: [u8; 8] = [0xaa, 0xbb, 0xcc, 0xdd, 0x11, 0x22, 0x33, 0x44];
            let mut stream = MockStream::new(data);
            let first: u32 = stream.read_stream_le().unwrap();
            let second = stream.into_le_iter().collect::<Vec<u32>>();

            assert_eq!(first, 0xddccbbaa);
            assert_eq!(second, [0x44332211]);
        }
    }

    mod into_be_iter {
        use super::*;

        #[test]
        fn should_iterate() {
            let data: [u8; 8] = [0xaa, 0xbb, 0xcc, 0xdd, 0x11, 0x22, 0x33, 0x44];
            let result = MockStream::new(data).into_be_iter().collect::<Vec<u32>>();
            assert_eq!(result, [0xaabbccdd, 0x11223344]);
        }

        #[test]
        fn should_iterate_from_cursor() {
            let data: [u8; 8] = [0xaa, 0xbb, 0xcc, 0xdd, 0x11, 0x22, 0x33, 0x44];
            let mut stream = MockStream::new(data);
            let first: u32 = stream.read_stream_be().unwrap();
            let second = stream.into_be_iter().collect::<Vec<u32>>();

            assert_eq!(first, 0xaabbccdd);
            assert_eq!(second, [0x11223344]);
        }
    }
}
