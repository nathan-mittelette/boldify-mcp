/// Converts an AST node to a Unicode-formatted String.
/// The style is determined by the `ContainerType` of the owning node.
pub trait ToUnicode {
    fn to_unicode(&self) -> String;
}
