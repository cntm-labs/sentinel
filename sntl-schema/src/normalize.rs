use sha2::{Digest, Sha256};

/// Normalize SQL deterministically: strip comments, collapse whitespace, trim.
/// String literals are preserved byte-for-byte.
pub fn normalize_sql(sql: &str) -> String {
    let stripped = strip_comments(sql);
    collapse_whitespace(&stripped)
}

/// SHA-256 hex digest of the normalized SQL. Truncated to 13 chars to match
/// the cache-file filename length specified in §6.1 of the design spec
/// (short enough to read, long enough for collision-free practical use).
pub fn hash_sql(sql: &str) -> String {
    let normalized = normalize_sql(sql);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(&digest[..7]);
    hex[..13].to_string()
}

fn strip_comments(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // String literals — pass through, handle '' escape
        if b == b'\'' {
            out.push(b);
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                        out.push(b'\'');
                        out.push(b'\'');
                        i += 2;
                        continue;
                    }
                    out.push(b'\'');
                    i += 1;
                    break;
                }
                out.push(bytes[i]);
                i += 1;
            }
            continue;
        }
        // Line comment --
        if b == b'-' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Block comment /* */
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(b);
        i += 1;
    }
    String::from_utf8(out).expect("input is utf-8, output is too")
}

fn collapse_whitespace(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut in_string = false;
    let mut prev_ws = false;
    for c in sql.chars() {
        if c == '\'' {
            in_string = !in_string;
            out.push(c);
            prev_ws = false;
            continue;
        }
        if in_string {
            out.push(c);
            continue;
        }
        if c.is_whitespace() {
            if !prev_ws && !out.is_empty() {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    out.trim().to_string()
}
