//! A minimal HTTP/1.0 POST over `std::net`, enough for a localhost runtime.
//! HTTP/1.0 keeps replies un-chunked, so the body is whatever follows the headers.

use crate::chat::{Reply, Transport};
use crate::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Instant;

pub struct HttpTransport;

/// Split `http://host:port/path` into (`host:port`, `/path`).
pub fn parse_url(url: &str) -> Result<(&str, &str), Error> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| Error::BadUrl(url.to_string()))?;
    match rest.find('/') {
        Some(i) if i > 0 => Ok((&rest[..i], &rest[i..])),
        Some(_) => Err(Error::BadUrl(url.to_string())),
        None if rest.is_empty() => Err(Error::BadUrl(url.to_string())),
        None => Ok((rest, "/")),
    }
}

/// Return the body of a 200 reply.
pub fn parse_response(raw: &str) -> Result<&str, Error> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| Error::BadResponse("no header terminator".into()))?;
    let code: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| Error::BadResponse("no status line".into()))?;
    if code != 200 {
        return Err(Error::Http(code));
    }
    Ok(body)
}

impl Transport for HttpTransport {
    fn post_json(&self, url: &str, body: &str) -> Result<Reply, Error> {
        let (host, path) = parse_url(url)?;
        let start = Instant::now();
        let mut s = TcpStream::connect(host).map_err(|e| Error::Io(e.to_string()))?;
        let req = format!(
            "POST {path} HTTP/1.0\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        s.write_all(req.as_bytes())
            .map_err(|e| Error::Io(e.to_string()))?;
        let mut raw = String::new();
        s.read_to_string(&mut raw)
            .map_err(|e| Error::Io(e.to_string()))?;
        let body = parse_response(&raw)?.to_string();
        Ok(Reply {
            body,
            elapsed: start.elapsed(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_cases() {
        assert_eq!(parse_url("http://h:1/a/b"), Ok(("h:1", "/a/b")));
        assert_eq!(parse_url("http://h:1"), Ok(("h:1", "/")));
        assert!(parse_url("https://h").is_err());
        assert!(parse_url("http://").is_err());
        assert!(parse_url("http:///x").is_err());
    }

    #[test]
    fn parse_response_cases() {
        assert_eq!(parse_response("HTTP/1.0 200 OK\r\n\r\nbody"), Ok("body"));
        assert_eq!(
            parse_response("HTTP/1.0 503 X\r\n\r\n"),
            Err(Error::Http(503))
        );
        assert!(matches!(
            parse_response("garbage"),
            Err(Error::BadResponse(_))
        ));
        assert!(matches!(
            parse_response("\r\n\r\nx"),
            Err(Error::BadResponse(_))
        ));
        assert!(matches!(
            parse_response("HTTP/1.0 abc\r\n\r\nx"),
            Err(Error::BadResponse(_))
        ));
    }
}
