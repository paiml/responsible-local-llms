//! The real transport against a one-shot loopback server.

use rll_client::http::HttpTransport;
use rll_client::{Error, Transport};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn serve_once(reply: &'static str) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut buf = [0u8; 4096];
        let _ = s.read(&mut buf).unwrap();
        s.write_all(reply.as_bytes()).unwrap();
    });
    format!("http://{addr}/v1/chat/completions")
}

#[test]
fn posts_and_reads_a_200() {
    let url = serve_once("HTTP/1.0 200 OK\r\n\r\n{\"ok\":true}");
    let r = HttpTransport.post_json(&url, "{}").unwrap();
    assert_eq!(r.body, "{\"ok\":true}");
}

#[test]
fn surfaces_a_non_200() {
    let url = serve_once("HTTP/1.0 500 Oops\r\n\r\n");
    assert_eq!(
        HttpTransport.post_json(&url, "{}").unwrap_err(),
        Error::Http(500)
    );
}

#[test]
fn bad_url_fails_before_connecting() {
    assert!(matches!(
        HttpTransport.post_json("ftp://x", "{}"),
        Err(Error::BadUrl(_))
    ));
}

#[test]
fn refused_connection_is_an_io_error() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    drop(l);
    let e = HttpTransport
        .post_json(&format!("http://{addr}/x"), "{}")
        .unwrap_err();
    assert!(matches!(e, Error::Io(_)));
}
