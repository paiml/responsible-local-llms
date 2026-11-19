//! The quality checker over the real HTTP transport, against a loopback server that
//! answers one prompt correctly and every other prompt wrongly.

use ch1_sovereign::quality;
use rll_client::{Config, Runtime};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn serve(n: usize) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    thread::spawn(move || {
        for _ in 0..n {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = [0u8; 8192];
            let got = s.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..got]).into_owned();
            let content = if req.contains("capital of France") {
                "Paris"
            } else {
                "no idea"
            };
            let body = format!(
                r#"{{"choices":[{{"message":{{"content":"{content}"}}}}],"usage":{{"completion_tokens":2}}}}"#
            );
            let _ = write!(s, "HTTP/1.0 200 OK\r\n\r\n{body}");
        }
    });
    format!("http://{addr}")
}

#[test]
fn live_scores_one_level_from_the_runtime() {
    let n = quality::parse_cases(quality::CASES_JSONL).unwrap().len();
    let cfg = Config {
        runtime: Runtime::LlamaCpp,
        base_url: serve(n),
        model: "m".into(),
    };
    let mut out = Vec::new();
    let s = quality::run(&cfg, true, "q8_0", &mut out).unwrap();
    assert!(s.contract_holds());
    assert_eq!(
        (s.scores[0].level.as_str(), s.scores[0].passed),
        ("q8_0", 1)
    );
    assert_eq!(s.scores[0].total, n);
    assert!(String::from_utf8(out).unwrap().contains("q8_0"));
}
