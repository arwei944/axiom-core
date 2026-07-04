use super::def::Witness;

#[derive(Debug, Clone)]
pub struct WitnessBatch {
    pub witnesses: Vec<Witness>,
}

impl WitnessBatch {
    pub fn new() -> Self {
        Self {
            witnesses: Vec::new(),
        }
    }

    pub fn push(&mut self, witness: Witness) {
        self.witnesses.push(witness);
    }

    pub fn is_empty(&self) -> bool {
        self.witnesses.is_empty()
    }

    pub fn len(&self) -> usize {
        self.witnesses.len()
    }

    pub fn verify_chain(&self) -> bool {
        Witness::verify_chain_integrity(&self.witnesses)
    }

    pub fn into_vec(self) -> Vec<Witness> {
        self.witnesses
    }
}

impl Default for WitnessBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for WitnessBatch {
    type Item = Witness;
    type IntoIter = std::vec::IntoIter<Witness>;
    fn into_iter(self) -> Self::IntoIter {
        self.witnesses.into_iter()
    }
}
