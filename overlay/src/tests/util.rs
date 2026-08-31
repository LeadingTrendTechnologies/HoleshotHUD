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
