use super::*;

#[test]
fn json_roundtrip_keeps_newlines() {
    let tickets = vec![Ticket {
        id: "abc".into(),
        kind: "bug".into(),
        summary: "It crashed".into(),
        sent_at: String::new(),
        reply: "Fixed in the next build.\nThanks.".into(),
        replied_at: String::new(),
        thread: Vec::new(),
        seen_reply: false,
    }];
    let text = serialize_tickets(&tickets);
    let back = parse_local_tickets(&text);
    assert_eq!(back, tickets);
}

#[test]
fn pending_skips_seen_and_empty() {
    *tickets_lock() = vec![
        Ticket {
            id: "a".into(),
            kind: "bug".into(),
            summary: "Old".into(),
            sent_at: String::new(),
            reply: "Already read".into(),
            replied_at: String::new(),
            thread: Vec::new(),
            seen_reply: true,
        },
        Ticket {
            id: "b".into(),
            kind: "feature".into(),
            summary: "Minimap zoom".into(),
            sent_at: String::new(),
            reply: "We added a slider.".into(),
            replied_at: String::new(),
            thread: Vec::new(),
            seen_reply: false,
        },
    ];
    let view = pending_reply().expect("unread");
    assert_eq!(view.id, "b");
    assert_eq!(view.kind_label, "Feature");
    assert_eq!(
        view.lines,
        vec![
            ChatLine {
                from_dev: false,
                text: "Minimap zoom".into(),
            },
            ChatLine {
                from_dev: true,
                text: "We added a slider.".into(),
            },
        ]
    );
    dismiss_reply("b");
    assert!(pending_reply().is_none());
}

#[test]
fn send_error_maps_status_text() {
    assert_eq!(
        send_error("HTTP 503"),
        "Couldn't send. Add FEEDBACK_GITHUB_TOKEN on Vercel."
    );
    assert_eq!(
        send_error("413 payload"),
        "Couldn't send. Race log was too large."
    );
    assert_eq!(
        send_error("gist failed"),
        "Couldn't send. Token needs gist access on Vercel."
    );
    assert_eq!(
        send_error("404 not found"),
        "Couldn't send. Deploy /api/feedback to Vercel."
    );
    assert_eq!(
        send_error("timed out"),
        "Couldn't send. No connection to the server."
    );
    assert_eq!(
        send_error("nope"),
        "Couldn't send. Report and log file copied."
    );
}

#[test]
fn merge_remote_reopens_unread_dev_reply() {
    *tickets_lock() = vec![Ticket {
        id: "b".into(),
        kind: "feature".into(),
        summary: "Minimap zoom".into(),
        sent_at: String::new(),
        reply: "We added a slider.".into(),
        replied_at: String::new(),
        thread: vec![Msg {
            from_dev: true,
            text: "We added a slider.".into(),
            at: String::new(),
        }],
        seen_reply: true,
    }];
    assert!(pending_reply().is_none());
    merge_remote(&[Ticket {
        id: "b".into(),
        kind: "feature".into(),
        summary: "Minimap zoom".into(),
        sent_at: String::new(),
        reply: "Which track?".into(),
        replied_at: String::new(),
        thread: vec![
            Msg {
                from_dev: true,
                text: "We added a slider.".into(),
                at: String::new(),
            },
            Msg {
                from_dev: true,
                text: "Which track?".into(),
                at: String::new(),
            },
        ],
        seen_reply: false,
    }]);
    let view = pending_reply().expect("unread again");
    assert_eq!(view.id, "b");
    assert_eq!(view.lines.last().unwrap().text, "Which track?");
    assert!(view.lines.last().unwrap().from_dev);
}

#[test]
fn parse_remote_array() {
    let body = r#"{"tickets":[{"id":"g1","kind":"bug","summary":"Hi","reply":"Hello\nthere","replied_at":"2026-01-01"}]}"#;
    let got = parse_remote_tickets(body);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, "g1");
    assert_eq!(got[0].reply, "Hello\nthere");
    assert_eq!(got[0].thread.len(), 1);
    assert!(got[0].thread[0].from_dev);
}

#[test]
fn parse_remote_thread() {
    let body = r#"{"tickets":[{"id":"g1","kind":"bug","summary":"Crash","thread":[{"from":"dev","text":"Which track?"},{"from":"user","text":"Hangtown"}]}]}"#;
    let got = parse_remote_tickets(body);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].thread.len(), 2);
    assert!(got[0].thread[0].from_dev);
    assert!(!got[0].thread[1].from_dev);
    assert_eq!(got[0].thread[1].text, "Hangtown");
}
