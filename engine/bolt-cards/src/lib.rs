//! Foundations (FDN) card definitions.
//!
//! [`supported`] is the source of truth for restricted drafts: a card is playable in the engine
//! only if it is listed here. Unsupported cards are listed, never skipped.

/// Names of the cards the engine fully implements, in build order.
pub fn supported() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_has_no_duplicates() {
        let mut names = supported().to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), supported().len());
    }
}
