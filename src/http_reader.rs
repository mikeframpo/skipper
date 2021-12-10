use reqwest::blocking::Client;
use reqwest::IntoUrl;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
enum HttpError {
    #[error("http: request error, cause: {source}")]
    RequestError {
        #[from]
        source: reqwest::Error,
    },

    #[error("http: unexpected response format, cause: {reason}")]
    FormatError { reason: String },
}

struct HttpReader {
    // TODO: include for range requests
    //content_length: u64,
    tmp_buf: Vec<u8>,
    tmp_remaining: usize,
}

impl HttpReader {
    pub fn new<T: IntoUrl>(url: T) -> Result<HttpReader, HttpError> {
        let client = Client::new();

        // TODO: implement range requests
        //let resp = client.head(url).send()?;
        //let content_length = resp.content_length().ok_or(HttpError::FormatError {
        //    reason: String::from("content length not returned in headers"),
        //})?;

        // TODO: need to check HTTP return code
        let resp = client.get(url).send()?;
        let buf = resp.bytes()?;

        Ok(HttpReader {
            tmp_buf: buf.to_vec(),
            tmp_remaining: buf.len(),
        })
    }
}

impl io::Read for HttpReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // DONE 1. read entire body into buffer, save to tmp
        //      get this working with the test server
        // 2. implement range requests, limiting buffer size
        // 3. implement more complex testing
        //      long delay in-between buffer fetch (infinite)
        // 4. handle X retries on failed buffer fetch before abort
        //      and configurable client timeouts
        // 5. possibly execute requests asynchronously

        let count = std::cmp::min(buf.len(), self.tmp_remaining);
        let offs = self.tmp_buf.len() - self.tmp_remaining;
        let write_count = std::io::copy(&mut &self.tmp_buf[offs..offs + count], &mut &mut *buf)?;
        self.tmp_remaining -= count;
        Ok(write_count as usize)
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
        let server = create_test_server("http-roots/test1");
        let url = format!("http://127.0.0.1:{}/test-file", server.port);

        let mut http_reader = HttpReader::new(url).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let count = http_reader.read_to_end(&mut buf).unwrap();

        assert_eq!(count, 1024);
    }
}
