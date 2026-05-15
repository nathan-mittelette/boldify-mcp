pub fn decompose_accent(c: char) -> Option<(char, &'static str)> {
    match c {
        'é' => Some(('e', "\u{0301}")),
        'è' => Some(('e', "\u{0300}")),
        'ê' => Some(('e', "\u{0302}")),
        'ë' => Some(('e', "\u{0308}")),
        'à' => Some(('a', "\u{0300}")),
        'á' => Some(('a', "\u{0301}")),
        'â' => Some(('a', "\u{0302}")),
        'ä' => Some(('a', "\u{0308}")),
        'ù' => Some(('u', "\u{0300}")),
        'ú' => Some(('u', "\u{0301}")),
        'û' => Some(('u', "\u{0302}")),
        'ü' => Some(('u', "\u{0308}")),
        'ô' => Some(('o', "\u{0302}")),
        'ö' => Some(('o', "\u{0308}")),
        'î' => Some(('i', "\u{0302}")),
        'ï' => Some(('i', "\u{0308}")),
        'ç' => Some(('c', "\u{0327}")),
        'œ' => None,
        'æ' => None,
        'É' => Some(('E', "\u{0301}")),
        'È' => Some(('E', "\u{0300}")),
        'Ê' => Some(('E', "\u{0302}")),
        'Ë' => Some(('E', "\u{0308}")),
        'À' => Some(('A', "\u{0300}")),
        'Á' => Some(('A', "\u{0301}")),
        'Â' => Some(('A', "\u{0302}")),
        'Ä' => Some(('A', "\u{0308}")),
        'Ù' => Some(('U', "\u{0300}")),
        'Ú' => Some(('U', "\u{0301}")),
        'Û' => Some(('U', "\u{0302}")),
        'Ü' => Some(('U', "\u{0308}")),
        'Ô' => Some(('O', "\u{0302}")),
        'Ö' => Some(('O', "\u{0308}")),
        'Î' => Some(('I', "\u{0302}")),
        'Ï' => Some(('I', "\u{0308}")),
        'Ç' => Some(('C', "\u{0327}")),
        'Œ' => None,
        'Æ' => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompose_e_acute() {
        assert_eq!(decompose_accent('é'), Some(('e', "\u{0301}")));
    }

    #[test]
    fn decompose_c_cedilla() {
        assert_eq!(decompose_accent('ç'), Some(('c', "\u{0327}")));
    }

    #[test]
    fn decompose_char_without_accent_returns_none() {
        assert_eq!(decompose_accent('z'), None);
        assert_eq!(decompose_accent('1'), None);
    }
}
