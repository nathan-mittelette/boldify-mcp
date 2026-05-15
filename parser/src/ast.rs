#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeBase {
    pub id: u64,
    pub span: Span,
}

impl NodeBase {
    pub fn new(id: u64, span: Span) -> Self {
        Self { id, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerType {
    Text,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Surline,
    Blockquote,
    List,
    OrderedList,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    pub base: NodeBase,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItemNode {
    pub base: NodeBase,
    pub children: Vec<InlineNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineNode {
    Text(TextNode),
    Container(ContainerNode),
    ListItem(ListItemNode),
}

impl InlineNode {
    pub fn base(&self) -> &NodeBase {
        match self {
            InlineNode::Text(n) => &n.base,
            InlineNode::Container(n) => &n.base,
            InlineNode::ListItem(n) => &n.base,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerNode {
    pub base: NodeBase,
    pub container_type: ContainerType,
    pub children: Vec<InlineNode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_len_computed_correctly() {
        let s = Span::new(3, 10);
        assert_eq!(s.len(), 7);
    }

    #[test]
    fn empty_span_detected() {
        let s = Span::new(5, 5);
        assert!(s.is_empty());
    }

    #[test]
    fn span_len_saturates_if_end_before_start() {
        // saturating_sub: no panic if end < start
        let s = Span::new(10, 5);
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
    }

    #[test]
    fn container_node_children_empty_by_default() {
        let node = ContainerNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            container_type: ContainerType::Text,
            children: vec![],
        };
        assert!(node.children.is_empty());
    }

    #[test]
    fn inline_node_base_returns_correct_reference() {
        let text_node = TextNode {
            base: NodeBase::new(42, Span::new(1, 5)),
            text: "hello".to_string(),
        };
        let inline = InlineNode::Text(text_node);
        assert_eq!(inline.base().id, 42);
        assert_eq!(inline.base().span.start, 1);
    }
}
