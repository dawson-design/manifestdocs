//! Token estimates, shared by `context`'s budget and `check`'s node size warning, so authors
//! can compare the two.

/// The estimated tokens in `text`: its characters divided by 3, rounded up.
pub fn estimate(text: &str) -> u32 {
    let chars = text.chars().count();
    u32::try_from(chars.div_ceil(3)).unwrap_or(u32::MAX)
}

/// `7100` as `7,100`.
pub fn grouped(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
