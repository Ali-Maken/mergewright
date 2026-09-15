//! Parsing the terminal's APC graphics replies back out of the input stream.

use super::GraphicsReply;

/// Every Kitty graphics reply in `bytes`; anything else is skipped.
#[must_use]
pub fn replies(bytes: &[u8]) -> Vec<GraphicsReply> {
    let mut found = Vec::new();
    let mut rest = bytes;
    while let Some(start) = rest.windows(3).position(|w| w == b"\x1b_G") {
        let body = &rest[start + 3..];
        let Some(end) = body.windows(2).position(|w| w == b"\x1b\\") else {
            break;
        };
        found.push(parse_reply(&body[..end]));
        rest = &body[end + 2..];
    }
    found
}

/// `i=…,p=…;message`.
fn parse_reply(body: &[u8]) -> GraphicsReply {
    let text = String::from_utf8_lossy(body);
    let (control, message) = text.split_once(';').unwrap_or((&text, ""));
    let key = |name: &str| {
        control
            .split(',')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value.parse().ok())
    };
    GraphicsReply {
        image: key("i"),
        placement: key("p"),
        message: message
            .chars()
            .map(|c| if c.is_control() { '\u{fffd}' } else { c })
            .collect(),
    }
}
