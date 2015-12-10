use std::io::{BufWriter, BufRead, Write};
use std::io;

pub struct ByteStreamSplitter<'a> {
    buffer: Vec<u8>,
    match_pointer: usize,
    peek_buffer: Vec<u8>,
    sperator: &'a [u8],
    input: &'a mut BufRead,
    started_splitting: bool,
    end_of_stream_reached: bool,
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



impl<'a> ByteStreamSplitter<'a> {
    pub fn new(input: &'a mut BufRead, sperator: &'a [u8]) -> ByteStreamSplitter<'a>{
        ByteStreamSplitter{
            input: input,
            match_pointer: 0,
            sperator: sperator,
            buffer: Vec::new(),
            peek_buffer:Vec::new(),
            started_splitting: false,
            end_of_stream_reached: false,
        }
    }

    pub fn read_until_next_matching_byte(&mut self) -> io::Result<usize> {
        self.input.read_until(self.sperator[self.match_pointer], &mut self.buffer)
    }

    pub fn next_to_buf(&mut self, output: &mut Write) -> SplitResult<SplitType>{

        if self.end_of_stream_reached {
            return Err(SplitError::Io(io::Error::new(io::ErrorKind::InvalidInput, "Stream has no more data.")));
        }

        let mut part_result: Option<SplitType> = Option::None;
        while part_result.is_none() {
            let num_bytes = try!(self.read_until_next_matching_byte());

            part_result = match num_bytes {
                0 => {
                    self.end_of_stream_reached = true;
                    try!(output.write_all(&self.peek_buffer));
                    self.peek_buffer.clear();
                    Some(SplitType::Suffix)
                },
                1 => {
                    self.peek_buffer.push(self.buffer[0]);

                    if self.match_pointer < self.sperator.len()-1 {
                        self.match_pointer +=1;
                        None
                    }else {
                        self.match_pointer = 0;
                        if self.started_splitting {
                            Some(SplitType::FullMatch)
                        } else {
                            self.started_splitting = true;
                            Some(SplitType::Prefix)
                        }
                    }
                },
                _ => {
                    try!(output.write_all(&self.peek_buffer));
                    self.peek_buffer.clear();

                    match self.match_pointer {
                        0 => {
                            self.match_pointer+=1;
                            self.peek_buffer.push(self.buffer[self.buffer.len()-1]);
                            try!(output.write_all(&self.buffer[..self.buffer.len()-1]));
                        },
                        _ => {
                            self.match_pointer=0;
                            try!(output.write_all(&self.buffer));
                        }
                    }
                    None
                }
            };
            self.buffer.clear();
        }
        part_result.ok_or(SplitError::Internal("Scan finished without succeeding or failing. This should never happen!".to_string()))
    }
}

impl <'a> Iterator for ByteStreamSplitter<'a> {
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
    let sperator = [0x00, 0x00];
    let mut data = io::Cursor::new(vec![
        0xAA, 0xAB,                     // Prefix
        0x00, 0x00, 0x01, 0x02, 0x03,   // FullMatch
        0x00, 0x00, 0x04, 0x05, 0x06,   // FullMatch
        0x00, 0x00, 0x07, 0x08          // Suffix
        ]);

    let mut splitter = ByteStreamSplitter::new(&mut data, &sperator);
    let prefix = splitter.next().unwrap().unwrap();
    let match1 = splitter.next().unwrap().unwrap();
    let match2 = splitter.next().unwrap().unwrap();
    let suffix = splitter.next().unwrap().unwrap();

    assert_eq!(prefix, vec![0xAA, 0xAB]);
    assert_eq!(match1, vec![0x00, 0x00, 0x01, 0x02, 0x03]);
    assert_eq!(match2, vec![0x00, 0x00, 0x04, 0x05, 0x06]);
    assert_eq!(suffix, vec![0x00, 0x00, 0x07, 0x08]);
}

#[test]
fn test_without_prefix() {
    let sperator = [0x00, 0x00];
    let mut data = io::Cursor::new(vec![
        0x00, 0x00, 0x01, 0x02, 0x03,   // FullMatch
        0x00, 0x00, 0x04, 0x05, 0x06,   // FullMatch
        0x00, 0x00, 0x07, 0x08          // Suffix
        ]);

    let mut splitter = ByteStreamSplitter::new(&mut data, &sperator);
    let prefix = splitter.next().unwrap().unwrap();
    let match1 = splitter.next().unwrap().unwrap();
    let match2 = splitter.next().unwrap().unwrap();
    let suffix = splitter.next().unwrap().unwrap();

    assert_eq!(prefix, vec![]);
    assert_eq!(match1, vec![0x00, 0x00, 0x01, 0x02, 0x03]);
    assert_eq!(match2, vec![0x00, 0x00, 0x04, 0x05, 0x06]);
    assert_eq!(suffix, vec![0x00, 0x00, 0x07, 0x08]);
}
