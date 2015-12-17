[![Build Status](https://travis-ci.org/forgemo/byte_stream_splitter.svg?branch=master)](https://travis-ci.org/forgemo/byte_stream_splitter)

# byte_stream_splitter
Rust library for splitting byte streams.


```rust
    // Prepare your separator sequence.
    let separator = [0x00, 0x00];        

    // Prepare your data byte stream.
    // This can be anything implementing the BufRead trait.
    let mut data = io::Cursor::new(vec![
        0xAA, 0xAB,                     // Prefix
        0x00, 0x00, 0x01, 0x02, 0x03,   // FullMatch
        0x00, 0x00, 0x04, 0x05, 0x06,   // FullMatch
        0x00, 0x00, 0x07, 0x08          // Suffix
        ]);

    // The splitter implements the Iterator trait and can be used as such.
    // You can iterate through the matches via next() or next_to_buf().
    // Use next() if you don't care about holding the whole match in memory while searching for the next separator.
    // Use next_to_buf() if you want to directly handle the matched bytes while scanning for the next separator.  

    // The first match contains all bytes until the first separator sequence is detected (Prefix).
    // The last match contains all bytes starting from the last detected separator sequence. (Suffix)
    // All other matches between the prefix and suffix contain all the bytes from a separator sequence until the next one starts.

    // Note: If the stream immediately starts with the separator, the prefix will still be returned empty.

    let mut splitter = ByteStreamSplitter::new(&mut data, &separator);
    let prefix = splitter.next().unwrap().unwrap();
    let match1 = splitter.next().unwrap().unwrap();
    let match2 = splitter.next().unwrap().unwrap();
    let suffix = splitter.next().unwrap().unwrap();

    assert_eq!(prefix, vec![0xAA, 0xAB]);
    assert_eq!(match1, vec![0x01, 0x02, 0x03]);
    assert_eq!(match2, vec![0x04, 0x05, 0x06]);
    assert_eq!(suffix, vec![0x07, 0x08]);
```
