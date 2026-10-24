//! Numbers as German readers expect them, shared by every page.

/// A number with a decimal comma and a real minus sign, e.g. "21,3" or "−2,1".
pub fn decimal(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
        .replace('.', ",")
        .replace('-', "\u{2212}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_a_decimal_comma() {
        assert_eq!(decimal(21.25, 1), "21,2");
        assert_eq!(decimal(1.5, 2), "1,50");
        assert_eq!(decimal(9.6, 0), "10");
    }

    #[test]
    fn uses_a_minus_sign() {
        assert_eq!(decimal(-2.14, 1), "\u{2212}2,1");
    }
}
