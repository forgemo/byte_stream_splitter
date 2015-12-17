use std::io::{BufWriter, BufRead, Write, Bytes};
use std::io;
use std::collections::VecDeque;

pub struct ByteStreamSplitter<'a,T: 'a> {
    separator: &'a [u8],
    input: Bytes<T>,
    started_splitting: bool,
    end_of_stream_reached: bool
}


pub type SplitResult<T> = Result<T,SplitError>;

#[derive(Debug)]
pub enum SplitType {
    Prefix,
    FullMatch,
    Suffix
}

#[derive(Debug)]
pub enum SplitError {
    Io(io::Error),
    Internal(String)
}

impl std::fmt::Display for SplitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            SplitError::Io(ref e) => e.fmt(f),
            SplitError::Internal(ref s) => write!(f,"{}", s)
        }
    }
}

impl std::error::Error for SplitError {
    fn description(&self) -> &str{
        match *self {
            SplitError::Io(ref e) => e.description(),
            SplitError::Internal(ref s) => s
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            SplitError::Io(ref e) => Some(e),
            SplitError::Internal(_) => None
        }
    }
}

impl From<io::Error> for SplitError {
    fn from(e: io::Error) -> Self {
        SplitError::Io(e)
    }
}



impl<'a, T> ByteStreamSplitter<'a,T> where T: BufRead + Sized{
    pub fn new(input: T, separator: &'a [u8]) -> ByteStreamSplitter<'a, T>{
        ByteStreamSplitter{
            input: input.bytes(),
            separator: separator,
            started_splitting: false,
            end_of_stream_reached: false
        }
    }

    pub fn next_to_buf(&mut self, output: &mut Write) -> SplitResult<SplitType>{

        if self.end_of_stream_reached {
            return Err(SplitError::Io(io::Error::new(io::ErrorKind::InvalidInput, "Stream has no more data.")));
        }

        let mut bytes = VecDeque::new();
        for b in self.input.by_ref().take(self.separator.len()) {
            bytes.push_back(try!(b));
        }
        while self.separator.iter().ne(bytes.iter()){
            let front_byte = try!(bytes.pop_front().ok_or(SplitError::Internal("This should never fail and must be a bug!".to_string())));
            try!(output.write(&[front_byte]));
            let next_byte = self.input.by_ref().next();
            if let Some(r) = next_byte {
                bytes.push_back(try!(r));
            }else {
                self.end_of_stream_reached = true;
                break;
            }
        }

        if self.end_of_stream_reached {
            try!(output.write_all(&bytes.into_iter().collect::<Vec<_>>()[..]));
            Ok(SplitType::Suffix)
        }else if self.started_splitting {
            Ok(SplitType::FullMatch)
        }else {
            self.started_splitting = true;
            Ok(SplitType::Prefix)
        }

    }
}

impl <'a,T> Iterator for ByteStreamSplitter<'a,T> where T: BufRead + Sized{
    type Item = SplitResult<Vec<u8>>;

    fn next(&mut self) -> Option<SplitResult<Vec<u8>>> {
        if self.end_of_stream_reached {
            None
        } else {
            let mut part = BufWriter::new(Vec::new());
            let result = self.next_to_buf(&mut part)
            .and(part.flush().map_err(SplitError::Io))
            .and(part.into_inner().map_err(|e|SplitError::Internal(e.to_string())));
            match result {
                Ok(inner) => Some(Ok(inner)),
                _   => None
            }
        }
   }
}


#[test]
fn test_with_prefix() {
    let separator = [0x00, 0x00];
    let mut data = io::Cursor::new(vec![
        0xAA, 0xAB,                     // Prefix
        0x00, 0x00, 0x01, 0x02, 0x03,   // FullMatch
        0x00, 0x00, 0x04, 0x05, 0x06,   // FullMatch
        0x00, 0x00, 0x07, 0x08          // Suffix
        ]);

    let mut splitter = ByteStreamSplitter::new(&mut data, &separator);
    let prefix = splitter.next().unwrap().unwrap();
    let match1 = splitter.next().unwrap().unwrap();
    let match2 = splitter.next().unwrap().unwrap();
    let suffix = splitter.next().unwrap().unwrap();

    assert_eq!(prefix, vec![0xAA, 0xAB]);
    assert_eq!(match1, vec![0x01, 0x02, 0x03]);
    assert_eq!(match2, vec![0x04, 0x05, 0x06]);
    assert_eq!(suffix, vec![0x07, 0x08]);
}

#[test]
fn test_without_prefix() {
    let separator = [0x00, 0x00];
    let mut data = io::Cursor::new(vec![
        0x00, 0x00, 0x01, 0x02, 0x03,   // FullMatch
        0x00, 0x00, 0x04, 0x05, 0x06,   // FullMatch
        0x00, 0x00, 0x07, 0x08          // Suffix
        ]);

    let mut splitter = ByteStreamSplitter::new(&mut data, &separator);
    let prefix = splitter.next().unwrap().unwrap();
    let match1 = splitter.next().unwrap().unwrap();
    let match2 = splitter.next().unwrap().unwrap();
    let suffix = splitter.next().unwrap().unwrap();

    assert_eq!(prefix, vec![]);
    assert_eq!(match1, vec![0x01, 0x02, 0x03]);
    assert_eq!(match2, vec![0x04, 0x05, 0x06]);
    assert_eq!(suffix, vec![0x07, 0x08]);
}

#[test]
fn test_skip_bug() {
    let separator = [0x00, 0x00];
    let mut data = io::Cursor::new(vec![
        0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x00, 0x00, 0x00, 0x04, 0x05, 0x06,
        0x00, 0x00, 0x07, 0x08
        ]);

    let mut splitter = ByteStreamSplitter::new(&mut data, &separator);
    let prefix = splitter.next().unwrap().unwrap();
    println!("p{:?}", prefix);
    let match1 = splitter.next().unwrap().unwrap();
    println!("1m{:?}", match1);
    let match2 = splitter.next().unwrap().unwrap();
    println!("1m{:?}", match2);
    let suffix = splitter.next().unwrap().unwrap();
    println!("s{:?}", suffix);


    assert_eq!(prefix, vec![]);
    assert_eq!(match1, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
    assert_eq!(match2, vec![0x00, 0x04, 0x05, 0x06]);
    assert_eq!(suffix, vec![0x07, 0x08]);
}
