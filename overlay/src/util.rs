use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if c.is_control() => {}
            c => o.push(c),
        }
    }
    o
}

pub fn json_string(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = &body[i + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if rest.starts_with("null") {
        return None;
    }
    if !rest.starts_with('"') {
        return None;
    }
    json_parse_string(&rest[1..])
}

pub fn json_bool(body: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = &body[i + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_parse_string(s: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next()? {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.len() != 4 {
                        return None;
                    }
                    let n = u32::from_str_radix(&hex, 16).ok()?;
                    out.push(char::from_u32(n)?);
                }
                _ => return None,
            }
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

pub fn json_array_slice<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = &body[i + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('[') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (j, &c) in bytes.iter().enumerate() {
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'[' {
            depth += 1;
        } else if c == b']' {
            depth -= 1;
            if depth == 0 {
                return rest.get(..j + 1);
            }
        }
    }
    None
}

pub fn hidden_powershell() -> Command {
    let mut cmd = Command::new("powershell.exe");
    cmd.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-WindowStyle",
        "Hidden",
    ])
    .creation_flags(CREATE_NO_WINDOW)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_string_unescapes_newlines() {
        let body = r#"{"reply":"Hello\nthere","id":"g1"}"#;
        assert_eq!(json_string(body, "reply").as_deref(), Some("Hello\nthere"));
        assert_eq!(json_string(body, "id").as_deref(), Some("g1"));
        assert_eq!(json_string(r#"{"reply":null}"#, "reply"), None);
        assert_eq!(json_bool(r#"{"seen_reply":true}"#, "seen_reply"), Some(true));
        assert_eq!(json_bool(r#"{"seen_reply":false}"#, "seen_reply"), Some(false));
        assert_eq!(json_bool(r#"{"seen_reply":1}"#, "seen_reply"), None);
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
        assert_eq!(json_string(r#"{"reply":"say \"hi\""}"#, "reply").as_deref(), Some("say \"hi\""));
        assert_eq!(json_string(r#"{"reply":"\u0041"}"#, "reply").as_deref(), Some("A"));
        assert_eq!(json_string(r#"{"n":123}"#, "n"), None);
        let arr = json_array_slice(
            r#"{"thread":[{"from":"dev","text":"Hi"},{"from":"user","text":"Yes"}]}"#,
            "thread",
        );
        assert_eq!(arr, Some(r#"[{"from":"dev","text":"Hi"},{"from":"user","text":"Yes"}]"#));
    }
}
