use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn apply_combining_mark(text: &str, mark: &str) -> String {
    text.graphemes(true)
        .map(|g| {
            if g == " " || g == "\n" {
                g.to_string()
            } else {
                format!("{}{}", g, mark)
            }
        })
        .collect()
}
