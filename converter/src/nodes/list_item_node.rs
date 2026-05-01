use parser::ListItemNode;

use crate::nodes::inline_node::inline_to_unicode;
use crate::traits::ToUnicode;

impl ToUnicode for ListItemNode {
    fn to_unicode(&self) -> String {
        self.children.iter().map(inline_to_unicode).collect()
    }
}
