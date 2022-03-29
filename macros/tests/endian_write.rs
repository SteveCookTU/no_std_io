use macros::EndianWrite;
use no_std_io::{Cursor, Error, StreamContainer, StreamWriter, Writer};

// This is here purely to test compilation
#[derive(Debug, Default, PartialEq, EndianWrite)]
struct StructWithGeneric<T: no_std_io::EndianRead + no_std_io::EndianWrite> {
    data: T,
}

#[derive(Debug, Default, PartialEq, EndianWrite)]
struct Test {
    first: u8,
    second: u32,
}

#[derive(Debug, Default, PartialEq)]
struct ListContainer<T: no_std_io::EndianWrite>(Vec<T>);

impl<T: no_std_io::EndianWrite> no_std_io::EndianWrite for ListContainer<T> {
    fn get_size(&self) -> usize {
        let mut size = 0;
        for item in self.0.iter() {
            size += item.get_size();
        }

        // 1 for the count
        size + 1
    }

    fn try_write_le(&self, dst: &mut [u8]) -> Result<usize, Error> {
        let size = self.get_size();
        if dst.len() < size {
            return Err(Error::InvalidSize {
                offset: 0,
                wanted_size: size,
                data_len: dst.len(),
            });
        }

        let mut stream = StreamContainer::new(dst);
        let count: u8 = self.0.len().try_into().unwrap();
        stream.write_stream_le(&count)?;

        for item in self.0.iter() {
            stream.write_stream_le(item)?;
        }

        Ok(stream.get_index())
    }

    fn try_write_be(&self, dst: &mut [u8]) -> Result<usize, Error> {
        let size = self.get_size();
        if dst.len() < size {
            return Err(Error::InvalidSize {
                offset: 0,
                wanted_size: size,
                data_len: dst.len(),
            });
        }

        let mut stream = StreamContainer::new(dst);
        let count: u8 = self.0.len().try_into().unwrap();
        stream.write_stream_be(&count)?;

        for item in self.0.iter() {
            stream.write_stream_be(item)?;
        }

        Ok(stream.get_index())
    }
}

#[derive(Debug, Default, PartialEq, EndianWrite)]
struct TestContainer {
    test: Test,
    list: ListContainer<u32>,
}

#[test]
fn should_write_le() {
    let value = Test {
        first: 0xaa,
        second: 0xeeddccbb,
    };
    let mut bytes = vec![0; 5];
    let result = bytes.write_le(0, &value).expect("Write should have worked");

    assert_eq!(result, 5);
    assert_eq!(bytes, [0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
}

#[test]
fn should_write_be() {
    let value = Test {
        first: 0xaa,
        second: 0xbbccddee,
    };
    let mut bytes = vec![0; 5];
    let result = bytes.write_be(0, &value).expect("Write should have worked");

    assert_eq!(result, 5);
    assert_eq!(bytes, [0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
}

#[test]
fn should_error_if_there_are_not_enough_bytes() {
    let value = Test {
        first: 0xaa,
        second: 0xeeddccbb,
    };
    let mut bytes: [u8; 4] = [0; 4];
    let result = bytes
        .write_le::<Test>(0, &value)
        .expect_err("This should have failed");

    assert_eq!(
        result,
        Error::InvalidSize {
            wanted_size: 4,
            offset: 0,
            data_len: 4
        }
    );
}

#[test]
fn should_write_dynamic_size_le() {
    let value = ListContainer::<u32>(vec![0x44332211, 0xddccbbaa]);
    let mut bytes = vec![];
    let result = bytes.write_le(0, &value).expect("Write should have worked");

    assert_eq!(result, 9);
    assert_eq!(
        bytes,
        [0x02, 0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]
    );
}

#[test]
fn should_write_dynamic_size_be() {
    let value = ListContainer::<u32>(vec![0x11223344, 0xaabbccdd]);
    let mut bytes = vec![];
    let result = bytes.write_be(0, &value).expect("Write should have worked");

    assert_eq!(result, 9);
    assert_eq!(
        bytes,
        [0x02, 0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]
    );
}

#[test]
fn should_write_nested_le() {
    let value = TestContainer {
        test: Test {
            first: 0x00,
            second: 0x44332211,
        },
        list: ListContainer(vec![0xddccbbaa, 0x88776655]),
    };
    let mut bytes = vec![];
    let result = bytes.write_le(0, &value).expect("Write should have worked");

    assert_eq!(result, 14);
    assert_eq!(
        bytes,
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0x55, 0x66, 0x77, 0x88,]
    )
}

#[test]
fn should_write_nested_be() {
    let value = TestContainer {
        test: Test {
            first: 0x00,
            second: 0x11223344,
        },
        list: ListContainer(vec![0xaabbccdd, 0x55667788]),
    };
    let mut bytes = vec![];
    let result = bytes.write_be(0, &value).expect("Write should have worked");

    assert_eq!(result, 14);
    assert_eq!(
        bytes,
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0x55, 0x66, 0x77, 0x88,]
    )
}
