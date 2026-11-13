//! Go の `%q`（strconv.Quote）と同じ引用。check の文言と `--where` のエラーが使う。

use std::fmt::Write;

/// 文字列を `"` で囲み、Go の strconv.Quote と同じく、印字できない文字をエスケープする。
///
/// 印字できるかは Go の unicode.IsPrint に合わせる（文字・記号・数字・句読点と ASCII の空白）。
/// Unicode の表は持たず、制御文字、ASCII の空白でない空白、書式の文字（Cf）、私用の領域を印字できないとする。
pub fn go_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\x07' => out.push_str("\\a"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x0b' => out.push_str("\\v"),
            c if is_print(c) => out.push(c),
            c if (c as u32) < 0x80 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c if (c as u32) < 0x10000 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => {
                let _ = write!(out, "\\U{:08x}", c as u32);
            }
        }
    }
    out.push('"');
    out
}

fn is_print(c: char) -> bool {
    if c == ' ' {
        return true;
    }
    if c.is_control() || c.is_whitespace() {
        return false;
    }
    !matches!(c as u32,
        // 書式の文字（Cf）
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890..=0x0891 | 0x08E2
        | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F
        | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD | 0x13430..=0x1343F
        | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0001 | 0xE0020..=0xE007F
        // 私用の領域
        | 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD
        // 文字でないもの
        | 0xFFFE..=0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_like_go() {
        assert_eq!(go_quote("done"), r#""done""#);
        assert_eq!(go_quote("a\"b\\c"), r#""a\"b\\c""#);
        assert_eq!(go_quote("献立 é"), "\"献立 é\"");
        assert_eq!(go_quote("\x01\t\u{7f}"), r#""\x01\t\x7f""#);
        assert_eq!(
            go_quote("\u{a0}\u{200b}\u{2028}"),
            "\"\\u00a0\\u200b\\u2028\""
        );
        assert_eq!(go_quote("\u{e0001}"), r#""\U000e0001""#);
    }
}
