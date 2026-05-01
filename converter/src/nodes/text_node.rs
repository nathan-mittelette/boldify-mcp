use parser::TextNode;

use crate::traits::ToUnicode;

impl ToUnicode for TextNode {
    fn to_unicode(&self) -> String {
        self.text.clone()
    }
}
