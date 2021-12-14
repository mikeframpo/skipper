use std::io;
use std::str::FromStr;
use std::time::Duration;
use log::*;
use reqwest::header::*;
use reqwest::blocking::Client;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("http: request error, cause: {source}")]
    RequestError {
        #[from]
        source: reqwest::Error,
    },

    #[error("http: unexpected response format, cause: {reason}")]
    FormatError { reason: String },
}

const CHUNK_SIZE: u64 = 1024;

struct RangeHeaderIterator {
    byte_pos: u64,
    content_length: u64,
}

impl Iterator for RangeHeaderIterator {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        let bytes_remaining = self.content_length - self.byte_pos;
        if bytes_remaining > 0 {
            let chunk = std::cmp::min(CHUNK_SIZE, bytes_remaining);
            let range = format!("bytes={}-{}", self.byte_pos, self.byte_pos + chunk);

            self.byte_pos += chunk;
            return Some(range);
        }
        None
    }
}

struct ChunkBuffer {
    buf: Vec<u8>,
    read_pos: usize,
}

impl ChunkBuffer {
    fn new(size: usize) -> ChunkBuffer {
        ChunkBuffer {
            buf: Vec::with_capacity(size),
            read_pos: 0,
        }
    }

    fn len(&self) -> usize {
        self.buf.len() - self.read_pos
    }

    fn read_bytes(&mut self, dest: &mut [u8]) -> usize {
        let count = std::cmp::min(dest.len(), self.buf.len());

        let mut src = &self.buf[self.read_pos..self.read_pos + count];
        let mut dest = dest;
        let count =
            std::io::copy(&mut src, &mut dest).expect("failed to copy chunk to dest") as usize;

        self.read_pos += count;
        count
    }

    fn write_bytes(&mut self, src: &[u8]) {
        // copy the entire src buffer, overwriting the existing content
        assert!(src.len() <= self.buf.capacity());
        self.buf.clear();
        self.buf.extend_from_slice(src);
        self.read_pos = 0;
    }
}

pub struct HttpReader {
    url: String,
    client: Client,
    ranges: RangeHeaderIterator,
    buf: ChunkBuffer,
}

impl HttpReader {
    pub fn new(url: &str, timeout: Duration) -> Result<HttpReader, HttpError> {
        let client_builder = Client::builder();
        let client = client_builder.timeout(timeout).build()?;

        // request headers
        let resp = client.head(url).send()?;
        let content_length = resp
            .headers()
            .get(CONTENT_LENGTH)
            .ok_or(HttpError::FormatError {
                reason: String::from(
                    "content length not returned in headers, this is required for Range requests",
                ),
            })?;
        let content_length = u64::from_str(content_length.to_str().unwrap()).map_err(|err| {
            HttpError::FormatError {
                reason: err.to_string(),
            }
        })?;

        Ok(HttpReader {
            url: String::from(url),
            client,
            ranges: RangeHeaderIterator {
                byte_pos: 0,
                content_length,
            },
            buf: ChunkBuffer::new(CHUNK_SIZE as usize),
        })
    }
}

impl io::Read for HttpReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // DONE 1. read entire body into buffer, save to tmp
        //      get this working with the test server
        // DONE 2. implement range requests, limiting buffer size
        // DONE 3. implement more complex testing
        //      - latency - delay in-between buffer fetch (infinite)
        // 4. handle X retries on failed buffer fetch before abort
        //      and configurable client timeouts
        // 5. possibly execute requests asynchronously,
        // 6. if doing async/threaded requests, make multiple range requests simultaneously

        if self.buf.len() > 0 {
            // return any remaining bytes in the buffer
            return Ok(self.buf.read_bytes(buf));
        }

        // otherwise, read the next range and request it
        match self.ranges.next() {
            Some(range) => {
                debug!("requesting next range: {}", range);

                let req = self
                    .client
                    .get(&self.url)
                    .header(RANGE, range)
                    .send()
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

                // copy the body to the chunk buffer
                self.buf.write_bytes(&req.bytes().unwrap());
                // copy the chunk buffer to the output
                return Ok(self.buf.read_bytes(buf));
            }
            None => {
                // all ranges read, return EOF
                return Ok(0);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_server::*;
    use crate::test_utils::*;
    use std::io::Read;

    #[test]
    fn test_read_to_end() {
        init_logging();
        let server_args = TestServerArgs::new("http-roots/test1");
        let server = create_test_server(server_args);

        let url = format!("http://127.0.0.1:{}/test-file", server.port);
        let mut http_reader = HttpReader::new(&url, Duration::from_secs(1)).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let count = http_reader.read_to_end(&mut buf).unwrap();

        assert_eq!(count, 1024);
    }

    #[test]
    fn test_timeout() {
        init_logging();
        let mut server_args = TestServerArgs::new("http-roots/test1");
        // negative latency is infinite
        server_args.response_latency(-1f32);
        let server = create_test_server(server_args);

        let url = format!("http://127.0.0.1:{}/test-file", server.port);
        let err = HttpReader::new(&url, Duration::from_secs(1))
            .err()
            .expect("expected reader to time out!");
        match err {
            HttpError::RequestError { source } => { assert!(source.is_timeout()) },
            _ => {}
        }
    }
}
