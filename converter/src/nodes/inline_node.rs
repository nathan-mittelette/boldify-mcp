use parser::InlineNode;

use crate::traits::ToUnicode;

pub fn inline_to_unicode(node: &InlineNode) -> String {
    match node {
        InlineNode::Text(n) => n.to_unicode(),
        InlineNode::Container(n) => n.to_unicode(),
        InlineNode::ListItem(n) => n.to_unicode(),
    }
}
