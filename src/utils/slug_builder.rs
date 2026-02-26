pub fn build_15m_slug(symbol: &str, period_start_unix: i64) -> String {
    format!("{}-updown-15m-{}", symbol.to_lowercase(), period_start_unix)
}

pub fn build_5m_slug(symbol: &str, period_start_unix: i64) -> String {
    format!("{}-updown-5m-{}", symbol.to_lowercase(), period_start_unix)
}

pub fn parse_price_to_beat_from_question(question: &str) -> Option<f64> {
    let q = question.to_lowercase();
    let idx = q.find("above ").or_else(|| q.find('$'))?;
    let after = &question[idx..];
    let mut num_start_byte = 0;
    for (i, c) in after.char_indices() {
        if c == '$' || c.is_ascii_digit() {
            num_start_byte = if c == '$' { i + c.len_utf8() } else { i };
            break;
        }
    }
    let num_str: String = after[num_start_byte..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .filter(|c| *c != ',')
        .collect();
    if num_str.is_empty() {
        return None;
    }
    num_str.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_slugs() {
        assert_eq!(build_15m_slug("BTC", 1700000000), "btc-updown-15m-1700000000");
        assert_eq!(build_5m_slug("Eth", 1700000300), "eth-updown-5m-1700000300");
    }

    #[test]
    fn parses_price_to_beat_from_question() {
        let question = "Will Bitcoin be above $97,500 at 10:15 ET?";
        assert_eq!(parse_price_to_beat_from_question(question), Some(97500.0));
    }
}
