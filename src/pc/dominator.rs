use super::*;

// FIXME: This is a bit different given our single-instruction-graph VS basic-blocks
//  it might be ideal to store intervals of labels, instead of every single instruction.

#[derive(Clone, Debug)]
pub struct DominatorFact {
    pub dominates: Option<Vec<Label>>,
}

impl Lattice for DominatorFact {
    fn bottom() -> Self {
        DominatorFact { dominates: None }
    }

    fn top() -> Self {
        Self::bottom()
    }

    fn join(&mut self, other: &Self, _label: Label) -> bool {
        if self.dominates.is_none() {
            self.dominates = other.dominates.clone();
            return true;
        }

        if let (Some(ref mut self_dominates), Some(ref other_dominates)) =
            (&mut self.dominates, &other.dominates)
        {
            for index in 0..self_dominates.len() {
                if self_dominates[index] != other_dominates[index] {
                    self_dominates.splice(index..self_dominates.len(), std::iter::empty());
                    return true;
                }
            }
        }

        false
    }
}

pub struct DominatorAnalysis;

impl<I: Instruction> Analysis<I, DominatorFact> for DominatorAnalysis {
    fn analyze(
        &mut self,
        _graph: &Graph<I>,
        label: Label,
        _instruction: &I,
        fact: &DominatorFact,
    ) -> Rewrite<I, DominatorFact> {
        let mut dominates = fact.dominates.clone();
        dominates.get_or_insert(vec![]).push(label);
        Rewrite::Fact(DominatorFact { dominates })
    }
}
