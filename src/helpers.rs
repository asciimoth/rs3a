use std::fmt::Write;

pub(crate) fn in_range(mut v: usize, start: usize, end: usize) -> usize {
    if v < start {
        v = start;
    }
    if v >= end {
        if end > 0 {
            v = end - 1;
        } else {
            v = 0;
        }
    }
    v
}

pub(crate) fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '/' => out.push_str("&#x2F;"),
            c => out.push(c),
        }
    }
    out
}

/// Given per-frame delays in milliseconds (len = N), produce:
/// - total duration in seconds (f64)
/// - `keyTimes` string (N+1 numbers separated by ';')
/// - `values` strings for each frame (Vec<String>, each has N+1 semicolon-separated items)
pub(crate) fn timing_for_svg(delays_ms: &[usize]) -> (f64, String, Vec<String>) {
    let total_ms: usize = delays_ms.iter().sum();
    let total_s = (total_ms as f64) / 1000.0;

    // cumulative times in ms: length = N+1
    let mut cum: Vec<usize> = Vec::with_capacity(delays_ms.len() + 1);
    let mut acc = 0usize;
    cum.push(acc);
    for &d in delays_ms {
        acc += d;
        cum.push(acc);
    }

    // keyTimes fractions as strings with fixed precision
    // We'll print 6 decimal places (fine for SVG); trim trailing zeros optional
    let key_times_parts: Vec<String> = cum
        .iter()
        .map(|&ms| {
            let frac = (ms as f64) / (total_ms as f64);
            // format with 6 decimals, remove trailing zeros for compactness
            let mut s = format!("{:.6}", frac);
            // trim trailing zeros and trailing dot
            while s.contains('.') && (s.ends_with('0') || s.ends_with('.')) {
                if s.ends_with('0') {
                    s.pop();
                } else if s.ends_with('.') {
                    s.pop();
                    break;
                }
            }
            if s.is_empty() {
                s = "0".into();
            }
            s
        })
        .collect();

    let key_times = key_times_parts.join(";");

    // build values per frame: for N frames we need N+1 items in each values
    let n = delays_ms.len();
    let mut values_vec: Vec<String> = Vec::with_capacity(n);
    for k in 0..n {
        // values[i] == "1" if i == k else "0"
        let mut parts = Vec::with_capacity(n + 1);
        for i in 0..=n {
            parts.push(if i == k { "1" } else { "0" }.to_string());
        }
        let joined = parts.join(";");
        values_vec.push(joined);
    }

    (total_s, key_times, values_vec)
}

/// Return a quoted JSON string (including the surrounding `"`).
/// - `"` and `\` are escaped.
/// - C0 controls (U+0000..U+001F) and C1 controls (U+007F..U+009F)
///   are encoded as `\uXXXX` (hex, 4 digits).
/// - All other valid Unicode scalar values are left as-is.
///
/// Example:
///   json_quote("Hello\x1B\n\"x\\") -> "\"Hello\\u001b\\u000a\\\"x\\\\\""
pub(crate) fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            // C0 controls and C1 controls -> \uXXXX
            '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}' => {
                // lower-case hex (you can use {:04X} for upper-case)
                write!(out, "\\u{:04x}", ch as u32).unwrap();
            }
            // Everything else: append as-is (valid UTF-8, JSON accepts Unicode)
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::escape_html;

    #[test]
    fn basic() {
        assert_eq!(escape_html("<&>\"'"), "&lt;&amp;&gt;&quot;&#x27;");
    }

    #[test]
    fn unicode_kept() {
        assert_eq!(escape_html("😀 & <"), "😀 &amp; &lt;");
    }

    #[test]
    fn already_escaped_becomes_double_escaped() {
        assert_eq!(escape_html("&amp;"), "&amp;amp;");
    }

    #[test]
    fn script_injection_becomes_safe() {
        assert_eq!(escape_html("</script>"), "&lt;&#x2F;script&gt;");
    }
}
