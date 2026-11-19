//! The typed tool interface: the model's text must parse into a [`ToolCall`] or nothing runs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Every tool the demos know, as the model must name it.
pub const TOOL_NAMES: [&str; 4] = ["add", "read_file", "write_file", "delete_file"];

/// One tool call. Unknown tools, missing fields, wrong types and extra fields all fail to parse.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolCall {
    Add { a: i64, b: i64 },
    ReadFile { path: String },
    WriteFile { path: String, content: String },
    DeleteFile { path: String },
}

impl ToolCall {
    /// Parse the first JSON object in the model's output (models wrap calls in prose).
    pub fn parse(text: &str) -> Result<ToolCall, String> {
        let body = text
            .find('{')
            .zip(text.rfind('}'))
            .and_then(|(s, e)| text.get(s..=e))
            .ok_or("no JSON object in the model output")?;
        serde_json::from_str(body).map_err(|e| format!("not a valid tool call: {e}"))
    }

    pub fn name(&self) -> &'static str {
        match self {
            ToolCall::Add { .. } => "add",
            ToolCall::ReadFile { .. } => "read_file",
            ToolCall::WriteFile { .. } => "write_file",
            ToolCall::DeleteFile { .. } => "delete_file",
        }
    }

    /// Calls that change the workspace.
    pub fn destructive(&self) -> bool {
        matches!(
            self,
            ToolCall::WriteFile { .. } | ToolCall::DeleteFile { .. }
        )
    }

    /// The path the call touches, if it touches one.
    pub fn path(&self) -> Option<&str> {
        match self {
            ToolCall::Add { .. } => None,
            ToolCall::ReadFile { path }
            | ToolCall::WriteFile { path, .. }
            | ToolCall::DeleteFile { path } => Some(path),
        }
    }

    /// The call as one line of JSON (for logs).
    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("a ToolCall always serializes")
    }
}

/// The files the tools act on: in memory, so a demo can never touch the real disk.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Workspace {
    files: BTreeMap<String, String>,
}

impl Workspace {
    pub fn new() -> Workspace {
        Workspace::default()
    }

    pub fn with_file(mut self, path: &str, content: &str) -> Workspace {
        self.files.insert(path.into(), content.into());
        self
    }

    pub fn files(&self) -> &BTreeMap<String, String> {
        &self.files
    }

    /// Run one call. `Err` is a tool failure (no such file, overflow), not a refusal.
    pub fn execute(&mut self, call: &ToolCall) -> Result<String, String> {
        match call {
            ToolCall::Add { a, b } => a
                .checked_add(*b)
                .map(|s| s.to_string())
                .ok_or_else(|| "overflow".to_string()),
            ToolCall::ReadFile { path } => self
                .files
                .get(path)
                .cloned()
                .ok_or_else(|| format!("no such file {path}")),
            ToolCall::WriteFile { path, content } => {
                self.files.insert(path.clone(), content.clone());
                Ok(format!("wrote {} bytes to {path}", content.len()))
            }
            ToolCall::DeleteFile { path } => self
                .files
                .remove(path)
                .map(|_| format!("deleted {path}"))
                .ok_or_else(|| format!("no such file {path}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_call_wrapped_in_prose() {
        let c = ToolCall::parse(r#"Sure. {"tool":"add","a":1,"b":2} done"#).unwrap();
        assert_eq!(c, ToolCall::Add { a: 1, b: 2 });
        assert_eq!(c.name(), "add");
        assert_eq!(c.path(), None);
        assert!(!c.destructive());
        assert_eq!(c.json(), r#"{"tool":"add","a":1,"b":2}"#);
    }

    #[test]
    fn refuses_everything_that_is_not_a_typed_call() {
        for bad in [
            "no braces at all",
            "} backwards {",
            r#"{"tool":"add""#,
            r#"{"tool":"format_disk"}"#,
            r#"{"tool":"add","a":"1","b":2}"#,
            r#"{"tool":"add","a":1}"#,
            r#"{"tool":"add","a":1,"b":2,"extra":true}"#,
            r#"{"a":1,"b":2}"#,
        ] {
            assert!(ToolCall::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn every_tool_name_parses_and_names_itself() {
        let calls = [
            r#"{"tool":"add","a":1,"b":1}"#,
            r#"{"tool":"read_file","path":"p"}"#,
            r#"{"tool":"write_file","path":"p","content":"c"}"#,
            r#"{"tool":"delete_file","path":"p"}"#,
        ];
        let names: Vec<&str> = calls
            .iter()
            .map(|c| ToolCall::parse(c).unwrap().name())
            .collect();
        assert_eq!(names, TOOL_NAMES);
        assert!(ToolCall::parse(calls[2]).unwrap().destructive());
        assert!(ToolCall::parse(calls[3]).unwrap().destructive());
        assert_eq!(ToolCall::parse(calls[1]).unwrap().path(), Some("p"));
    }

    #[test]
    fn the_workspace_runs_each_tool_and_reports_failures() {
        let mut ws = Workspace::new().with_file("a", "hi");
        assert_eq!(ws.execute(&ToolCall::Add { a: 2, b: 3 }), Ok("5".into()));
        let max = ToolCall::Add { a: i64::MAX, b: 1 };
        assert_eq!(ws.execute(&max), Err("overflow".into()));
        let read = |p: &str| ToolCall::ReadFile { path: p.into() };
        assert_eq!(ws.execute(&read("a")), Ok("hi".into()));
        assert!(ws.execute(&read("zz")).is_err());
        let write = ToolCall::WriteFile {
            path: "b".into(),
            content: "xyz".into(),
        };
        assert_eq!(ws.execute(&write), Ok("wrote 3 bytes to b".into()));
        assert_eq!(ws.files().get("b").map(String::as_str), Some("xyz"));
        let del = |p: &str| ToolCall::DeleteFile { path: p.into() };
        assert_eq!(ws.execute(&del("b")), Ok("deleted b".into()));
        assert!(ws.execute(&del("b")).is_err());
        assert_eq!(ws.files().len(), 1);
    }
}
