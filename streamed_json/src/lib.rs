use serde::de::DeserializeOwned;
use serde_json::{self, Deserializer};
use std::io::{self, Read};

pub trait StreamedJsonDeserializable {
    fn deserialize_for_field<R: Read>(reader: R, field_name: Option<&str>) -> io::Result<Self>
    where
        Self: Sized;

}

pub trait ReaderDeserializableExt: DeserializeOwned {
    fn deserialize_single<R: Read>(reader: R) -> io::Result<Self>
    where
        Self: Sized;
}

impl<T> ReaderDeserializableExt for T
where
    T: DeserializeOwned,
{
    fn deserialize_single<R: Read>(reader: R) -> io::Result<Self> {
        let mut deserializer = Deserializer::from_reader(reader);
        let value = Self::deserialize(&mut deserializer)?;
        Ok(value)
    }
}

fn read_skipping_ws(mut reader: impl Read) -> io::Result<u8> {
    loop {
        let mut byte = 0u8;
        reader.read_exact(std::slice::from_mut(&mut byte))?;
        if !byte.is_ascii_whitespace() {
            return Ok(byte);
        }
    }
}

fn invalid_data(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

fn yield_next_obj<T: StreamedJsonDeserializable, R: Read>(
    mut reader: R,
    current_field: &mut Option<String>
) -> io::Result<Option<T>> {
    if current_field.is_none() {
        read_till_next_key(&mut reader, current_field)?;

        let peek = read_skipping_ws(&mut reader)?;
        if peek == b']' {
            *current_field = None;
            yield_next_obj(reader, current_field)
        } else {
            T::deserialize_for_field(io::Cursor::new([peek]).chain(reader), current_field.as_deref()).map(Some)
        }
    } else {
        match read_skipping_ws(&mut reader) {
            Ok(b',') => T::deserialize_for_field(reader, current_field.as_deref()).map(Some),
            Ok(b']') => {
                *current_field = None;
                yield_next_obj(reader, current_field)
            },
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
            Ok(v) => {
                T::deserialize_for_field(io::Cursor::new([v]).chain(reader), current_field.as_deref()).map(Some)
            },
            Err(e) => Err(e),
        }
    }
}


pub fn iter_json_array<R: Read, B: StreamedJsonDeserializable>(
    mut reader: R,
) -> impl Iterator<Item = Result<B, io::Error>> {
    let mut current_field = None;

    std::iter::from_fn(move || {
        let res = yield_next_obj(reader.by_ref(), &mut current_field);
        match res {
            Ok(Some(obj)) => {
                Some(Ok(obj))
            }
            Ok(None) => {
                None
            },
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof || e.kind() == io::ErrorKind::InvalidData => {
                None
            }
            Err(e) => {
                Some(Err(e))
            }
        }
        
    })
}

fn read_till_next_key<R: Read>(reader: &mut R, current_field: &mut Option<String>) -> io::Result<()> {
    // Skip to first [ and store the key before the array which is "key":
    let mut tracking_key = false;
    let mut key = String::new();

    loop {
        let byte = read_skipping_ws(&mut *reader)?;

        if byte == b'"' {
            if tracking_key {
                tracking_key = false;
                *current_field = Some(key.clone());
                key.clear();
            } else {
                tracking_key = true;
            }
        } else if tracking_key {
            key.push(byte as char);
        }

        if byte == b'[' {
            return Ok(())
        }
    }
}

pub use streamed_json_proc_macro::streamed_json;