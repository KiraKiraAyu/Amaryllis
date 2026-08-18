/// Incremental SSE line reader over raw bytes.
///
/// Network chunks can split a multi-byte UTF-8 character at an arbitrary
/// boundary, so lines must be reassembled as bytes and decoded only once a
/// complete line (terminated by `\n`) is available.
#[derive(Default)]
pub(super) struct SseLineReader {
    buf: Vec<u8>,
}

impl SseLineReader {
    pub(super) fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Returns the next complete line without the trailing newline, or
    /// `None` while no full line has arrived. A trailing `\r` (CRLF framing)
    /// and surrounding whitespace are trimmed.
    pub(super) fn next_line(&mut self) -> Option<String> {
        let newline = self.buf.iter().position(|&byte| byte == b'\n')?;
        let mut line: Vec<u8> = self.buf.drain(..=newline).collect();
        line.pop();
        Some(String::from_utf8_lossy(&line).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_multibyte_chars_split_across_chunks() {
        // "你" is E4 BD A0; the first chunk cuts between BD and A0.
        let mut reader = SseLineReader::default();
        reader.push(&[0xE4, 0xBD]);
        assert_eq!(reader.next_line(), None);

        reader.push(&[0xA0, b'\n', b'o', b'k', b'\n']);
        assert_eq!(reader.next_line(), Some("你".to_string()));
        assert_eq!(reader.next_line(), Some("ok".to_string()));
        assert_eq!(reader.next_line(), None);
    }

    #[test]
    fn yields_lines_only_when_complete() {
        let mut reader = SseLineReader::default();
        reader.push(b"data: {\"a\"");
        assert_eq!(reader.next_line(), None);

        reader.push(b":1}\ndata: [DONE]\n\n");
        assert_eq!(reader.next_line(), Some("data: {\"a\":1}".to_string()));
        assert_eq!(reader.next_line(), Some("data: [DONE]".to_string()));
        assert_eq!(reader.next_line(), Some(String::new()));
        assert_eq!(reader.next_line(), None);
    }

    #[test]
    fn trims_crlf_framing() {
        let mut reader = SseLineReader::default();
        reader.push(b"event: delta\r\ndata: {}\r\n");
        assert_eq!(reader.next_line(), Some("event: delta".to_string()));
        assert_eq!(reader.next_line(), Some("data: {}".to_string()));
        assert_eq!(reader.next_line(), None);
    }
}
