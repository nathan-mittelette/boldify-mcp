pub struct NodeIdGen(u64);

impl NodeIdGen {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn next_id(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

impl Default for NodeIdGen {
    fn default() -> Self {
        Self::new()
    }
}
