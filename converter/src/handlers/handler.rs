pub trait FontHandler: Send + Sync {
    /// Converts plain text to the target Unicode style.
    /// Spaces and newlines (\n) are preserved as-is.
    fn apply(&self, text: &str) -> String;
}
