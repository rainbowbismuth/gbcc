use super::forward_analysis::*;
use super::graph::{Entry, Graph, Label, Language};
use super::lattice::Lattice;

#[derive(Clone, Debug)]
pub struct DominatorFact {
    pub dominates: Option<Vec<Label>>,
}

impl Lattice for DominatorFact {
    fn bottom() -> Self {
        DominatorFact { dominates: None }
    }

    fn join(&mut self, other: &Self, label: Label) -> bool {
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

impl<L: Language> ForwardAnalysis<L, DominatorFact> for DominatorAnalysis {
    fn analyze_entry(
        &mut self,
        _graph: &Graph<L>,
        label: Label,
        entry: &L::Entry,
        mut fact: DominatorFact,
    ) -> DominatorFact {
        fact.dominates.get_or_insert(vec![]).push(entry.label());
        fact
    }

    fn analyze_instruction(
        &mut self,
        _graph: &Graph<L>,
        _label: Label,
        _instruction: &L::Instruction,
        _analyze: AnalyzeInstruction<DominatorFact>,
    ) -> Option<RewriteInstruction<L>> {
        None
    }

    fn analyze_exit(
        &mut self,
        _graph: &Graph<L>,
        _label: Label,
        exit: &L::Exit,
        fact: &DominatorFact,
    ) -> RewriteExit<L, DominatorFact> {
        RewriteExit::Done(distribute_facts::<L, DominatorFact>(exit, fact))
    }
}
