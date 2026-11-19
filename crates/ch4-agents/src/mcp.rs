//! Lesson 4.2: tools over MCP. A minimal MCP-style server and client: JSON-RPC 2.0, one message
//! per line, over a TCP socket bound to 127.0.0.1 only. It speaks `initialize`, `tools/list` and
//! `tools/call`, the subset an agent needs; real MCP servers also use stdio or HTTP framing, and
//! this socket framing exists so the test needs no network beyond loopback.

use crate::tool::{ToolCall, Workspace, TOOL_NAMES};
use serde_json::{json, Map, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

const PROTOCOL: &str = "2024-11-05";

fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn text_result(text: &str, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

/// The `tools/list` payload: each tool with a JSON schema for its arguments.
pub fn tools_list() -> Value {
    let args = |names: &[&str]| -> Value {
        let props: Map<String, Value> = names
            .iter()
            .map(|n| {
                let ty = if matches!(*n, "a" | "b") {
                    "integer"
                } else {
                    "string"
                };
                (n.to_string(), json!({"type": ty}))
            })
            .collect();
        json!({"type": "object", "properties": props, "required": names, "additionalProperties": false})
    };
    json!([
        {"name": TOOL_NAMES[0], "description": "Add two integers.", "inputSchema": args(&["a", "b"])},
        {"name": TOOL_NAMES[1], "description": "Read a file.", "inputSchema": args(&["path"])},
        {"name": TOOL_NAMES[2], "description": "Write a file.", "inputSchema": args(&["path", "content"])},
        {"name": TOOL_NAMES[3], "description": "Delete a file.", "inputSchema": args(&["path"])}
    ])
}

fn call(id: Value, params: &Value, ws: &mut Workspace) -> Value {
    let name = params.get("name").and_then(Value::as_str);
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let (Some(name), Value::Object(mut map)) = (name, args) else {
        return error(id, -32602, "params need a name and an arguments object");
    };
    map.insert("tool".into(), Value::String(name.into()));
    match serde_json::from_value::<ToolCall>(Value::Object(map)) {
        Err(e) => error(id, -32602, &format!("invalid tool call: {e}")),
        Ok(c) => match ws.execute(&c) {
            Ok(text) => ok(id, text_result(&text, false)),
            Err(text) => ok(id, text_result(&text, true)),
        },
    }
}

/// Answer one request line with one response line (no newline).
pub fn handle(line: &str, ws: &mut Workspace) -> String {
    let reply = match serde_json::from_str::<Value>(line) {
        Err(_) => error(Value::Null, -32700, "parse error"),
        Ok(req) => {
            let id = req.get("id").cloned().unwrap_or(Value::Null);
            let params = req.get("params").cloned().unwrap_or(Value::Null);
            match req.get("method").and_then(Value::as_str) {
                Some("initialize") => ok(
                    id,
                    json!({"protocolVersion": PROTOCOL, "capabilities": {"tools": {}},
                           "serverInfo": {"name": "rll-mcp", "version": "0.1.0"}}),
                ),
                Some("tools/list") => ok(id, json!({"tools": tools_list()})),
                Some("tools/call") => call(id, &params, ws),
                _ => error(id, -32601, "method not found"),
            }
        }
    };
    reply.to_string()
}

/// A server bound to loopback. It serves one connection, then returns.
pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn bind_loopback() -> io::Result<Server> {
        Ok(Server {
            listener: TcpListener::bind(("127.0.0.1", 0))?,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("a bound listener has an address")
    }

    /// Accept one client and answer every line until it hangs up. Returns how many requests it
    /// answered and the workspace as the tools left it.
    pub fn serve_one(self, mut ws: Workspace) -> io::Result<(usize, Workspace)> {
        let (stream, _) = self.listener.accept()?;
        let mut out = stream.try_clone()?;
        let mut answered = 0;
        for line in BufReader::new(stream).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            writeln!(out, "{}", handle(&line, &mut ws))?;
            answered += 1;
        }
        Ok((answered, ws))
    }
}

/// The agent's side of the connection.
pub struct McpClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    next_id: u64,
}

impl McpClient {
    /// Connect to a loopback address; anything else is refused before a socket opens.
    pub fn connect(addr: SocketAddr) -> io::Result<McpClient> {
        if !addr.ip().is_loopback() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the demo MCP client only talks to loopback",
            ));
        }
        let writer = TcpStream::connect(addr)?;
        Ok(McpClient {
            reader: BufReader::new(writer.try_clone()?),
            writer,
            next_id: 1,
        })
    }

    /// Send one raw line and read one reply line.
    pub fn raw(&mut self, line: &str) -> io::Result<Value> {
        writeln!(self.writer, "{line}")?;
        let mut reply = String::new();
        self.reader.read_line(&mut reply)?;
        serde_json::from_str(&reply).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn request(&mut self, method: &str, params: Value) -> io::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.raw(&req.to_string())
    }

    /// `tools/list`, reduced to the tool names.
    pub fn list_tools(&mut self) -> io::Result<Vec<String>> {
        let reply = self.request("tools/list", json!({}))?;
        Ok(reply["result"]["tools"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default())
    }

    /// `tools/call` for a typed call. The outer `Result` is the transport and protocol, the inner
    /// one is the tool: `Err` carries the tool's own error text (`isError`).
    pub fn call_tool(&mut self, call: &ToolCall) -> io::Result<Result<String, String>> {
        let mut args = serde_json::to_value(call).expect("a ToolCall always serializes");
        let name = args
            .as_object_mut()
            .and_then(|m| m.remove("tool"))
            .unwrap_or(Value::Null);
        let reply = self.request("tools/call", json!({"name": name, "arguments": args}))?;
        if let Some(e) = reply.get("error") {
            return Err(io::Error::other(e["message"].to_string()));
        }
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        Ok(if reply["result"]["isError"] == json!(true) {
            Err(text)
        } else {
            Ok(text)
        })
    }
}

/// What the lesson measures.
pub struct Summary {
    pub loopback: bool,
    pub listed: Vec<String>,
    pub add_text: String,
    pub read_text: String,
    pub missing_is_tool_error: bool,
    pub unknown_tool_code: i64,
    pub garbage_code: i64,
    pub sent: usize,
    pub served: usize,
    pub refused_remote: bool,
}

impl Summary {
    /// The server is loopback-only, lists exactly the typed tools, runs them with the same
    /// answers the local agent gets, reports a tool failure as a tool error, refuses an unknown
    /// tool and garbage with JSON-RPC errors, and answers every request it was sent.
    pub fn contract_holds(&self) -> bool {
        self.loopback
            && self.listed == TOOL_NAMES
            && self.add_text == "42"
            && self.read_text == "hello"
            && self.missing_is_tool_error
            && self.unknown_tool_code == -32602
            && self.garbage_code == -32700
            && self.sent == self.served
            && self.refused_remote
    }
}

/// Run a server on a thread and an agent against it, all on 127.0.0.1.
pub fn run(out: &mut impl Write) -> io::Result<Summary> {
    let server = Server::bind_loopback()?;
    let addr = server.addr();
    let ws = Workspace::new().with_file("notes.txt", "hello");
    let handle = std::thread::spawn(move || server.serve_one(ws));
    let remote: SocketAddr = "192.0.2.1:9".parse().expect("a literal address parses");
    let refused_remote = McpClient::connect(remote).is_err();
    let mut c = McpClient::connect(addr)?;
    let init = c.request("initialize", json!({}))?;
    let _ = writeln!(out, "server     : {}", init["result"]["serverInfo"]);
    let listed = c.list_tools()?;
    let _ = writeln!(out, "tools/list : {}", listed.join(", "));
    // The model's recorded output; the agent parses it and sends it over MCP.
    let model_output = r#"{"tool":"add","a":17,"b":25}"#;
    let add = ToolCall::parse(model_output).expect("the recorded call parses");
    let add_text = c.call_tool(&add)?.unwrap_or_default();
    let _ = writeln!(out, "add        : {add_text}");
    let read = ToolCall::ReadFile {
        path: "notes.txt".into(),
    };
    let read_text = c.call_tool(&read)?.unwrap_or_default();
    let _ = writeln!(out, "read_file  : {read_text}");
    let missing = ToolCall::ReadFile {
        path: "gone.txt".into(),
    };
    let missing_is_tool_error = c.call_tool(&missing)?.is_err();
    let _ = writeln!(out, "gone.txt   : tool error = {missing_is_tool_error}");
    let unknown = c.request(
        "tools/call",
        json!({"name": "format_disk", "arguments": {}}),
    )?;
    let unknown_tool_code = unknown["error"]["code"].as_i64().unwrap_or_default();
    let _ = writeln!(out, "format_disk: JSON-RPC error {unknown_tool_code}");
    let garbage_code = c.raw("not json")?["error"]["code"]
        .as_i64()
        .unwrap_or_default();
    let _ = writeln!(out, "garbage    : JSON-RPC error {garbage_code}");
    drop(c);
    let (served, _) = handle.join().expect("the server thread does not panic")?;
    Ok(Summary {
        loopback: addr.ip().is_loopback(),
        listed,
        add_text,
        read_text,
        missing_is_tool_error,
        unknown_tool_code,
        garbage_code,
        sent: 7,
        served,
        refused_remote,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpc(line: &str, ws: &mut Workspace) -> Value {
        serde_json::from_str(&handle(line, ws)).unwrap()
    }

    #[test]
    fn handle_answers_each_method_and_each_error() {
        let mut ws = Workspace::new();
        let init = rpc(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#, &mut ws);
        assert_eq!(init["result"]["protocolVersion"], PROTOCOL);
        let list = rpc(r#"{"id":2,"method":"tools/list"}"#, &mut ws);
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 4);
        assert_eq!(
            list["result"]["tools"][0]["inputSchema"]["properties"]["a"]["type"],
            "integer"
        );
        assert_eq!(
            rpc(r#"{"id":3,"method":"nope"}"#, &mut ws)["error"]["code"],
            -32601
        );
        assert_eq!(rpc("{", &mut ws)["error"]["code"], -32700);
        assert_eq!(rpc("{}", &mut ws)["error"]["code"], -32601);
    }

    #[test]
    fn tools_call_runs_a_tool_and_reports_bad_params() {
        let mut ws = Workspace::new();
        let add = rpc(
            r#"{"id":1,"method":"tools/call","params":{"name":"add","arguments":{"a":2,"b":3}}}"#,
            &mut ws,
        );
        assert_eq!(add["result"]["content"][0]["text"], "5");
        assert_eq!(add["result"]["isError"], false);
        let gone = rpc(
            r#"{"id":2,"method":"tools/call","params":{"name":"read_file","arguments":{"path":"x"}}}"#,
            &mut ws,
        );
        assert_eq!(gone["result"]["isError"], true);
        for bad in [
            r#"{"id":3,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#,
            r#"{"id":4,"method":"tools/call","params":{"name":"add","arguments":{"a":"x","b":1}}}"#,
            r#"{"id":5,"method":"tools/call","params":{"arguments":{}}}"#,
            r#"{"id":6,"method":"tools/call","params":{"name":"add","arguments":7}}"#,
            r#"{"id":7,"method":"tools/call"}"#,
        ] {
            assert_eq!(rpc(bad, &mut ws)["error"]["code"], -32602, "{bad}");
        }
    }

    #[test]
    fn a_remote_address_is_refused_before_connecting() {
        let remote: SocketAddr = "192.0.2.1:9".parse().unwrap();
        let err = McpClient::connect(remote).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn a_blank_line_gets_no_answer_and_the_server_returns_the_workspace() {
        let server = Server::bind_loopback().unwrap();
        let addr = server.addr();
        let t = std::thread::spawn(move || server.serve_one(Workspace::new()));
        let mut c = McpClient::connect(addr).unwrap();
        writeln!(c.writer, "   ").unwrap();
        let w = ToolCall::WriteFile {
            path: "f".into(),
            content: "c".into(),
        };
        assert_eq!(c.call_tool(&w).unwrap(), Ok("wrote 1 bytes to f".into()));
        drop(c);
        let (n, ws) = t.join().unwrap().unwrap();
        assert_eq!(n, 1);
        assert_eq!(ws.files().len(), 1);
    }

    #[test]
    fn a_hung_up_server_is_an_error_not_a_hang() {
        let server = Server::bind_loopback().unwrap();
        let addr = server.addr();
        let t = std::thread::spawn(move || {
            let (s, _) = server.listener.accept().unwrap();
            drop(s);
        });
        let mut c = McpClient::connect(addr).unwrap();
        t.join().unwrap();
        assert!(c.raw("{}").is_err());
    }

    #[test]
    fn a_reply_that_is_not_json_is_invalid_data() {
        let server = Server::bind_loopback().unwrap();
        let addr = server.addr();
        let t = std::thread::spawn(move || {
            let (s, _) = server.listener.accept().unwrap();
            let mut out = s.try_clone().unwrap();
            let mut line = String::new();
            BufReader::new(s).read_line(&mut line).unwrap();
            writeln!(out, "not json").unwrap();
        });
        let mut c = McpClient::connect(addr).unwrap();
        let e = c.raw("{}").unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        t.join().unwrap();
    }

    #[test]
    fn a_protocol_error_reaches_call_tool_as_an_error() {
        let server = Server::bind_loopback().unwrap();
        let addr = server.addr();
        let t = std::thread::spawn(move || {
            let (s, _) = server.listener.accept().unwrap();
            let mut out = s.try_clone().unwrap();
            let mut line = String::new();
            BufReader::new(s).read_line(&mut line).unwrap();
            writeln!(out, "{}", error(Value::Null, -32000, "boom")).unwrap();
        });
        let mut c = McpClient::connect(addr).unwrap();
        let call = ToolCall::Add { a: 1, b: 1 };
        assert!(c.call_tool(&call).is_err());
        t.join().unwrap();
        // With no tools array in the reply, the list is empty rather than a panic.
        let server = Server::bind_loopback().unwrap();
        let addr = server.addr();
        let t = std::thread::spawn(move || {
            let (s, _) = server.listener.accept().unwrap();
            let mut out = s.try_clone().unwrap();
            let mut line = String::new();
            BufReader::new(s).read_line(&mut line).unwrap();
            writeln!(out, "{}", ok(Value::Null, json!({}))).unwrap();
        });
        let mut c = McpClient::connect(addr).unwrap();
        assert!(c.list_tools().unwrap().is_empty());
        t.join().unwrap();
    }

    #[test]
    fn the_lesson_run_holds_its_contract_and_each_clause_can_fail() {
        let mut buf = Vec::new();
        let s = run(&mut buf).unwrap();
        assert!(s.contract_holds());
        assert!(String::from_utf8(buf)
            .unwrap()
            .contains("tools/list : add, read_file"));
        type Break = fn(&mut Summary);
        let breaks: [Break; 9] = [
            |s| s.loopback = false,
            |s| {
                s.listed.pop();
            },
            |s| s.add_text = "41".into(),
            |s| s.read_text = "x".into(),
            |s| s.missing_is_tool_error = false,
            |s| s.unknown_tool_code = 0,
            |s| s.garbage_code = 0,
            |s| s.served -= 1,
            |s| s.refused_remote = false,
        ];
        for b in breaks {
            let mut broken = run(&mut Vec::new()).unwrap();
            b(&mut broken);
            assert!(!broken.contract_holds());
        }
    }
}
