use super::*;

#[derive(Clone, Debug)]
pub struct FactBase<F> {
    // Like a Graph, there's a sub_graph vector and then a fact per instruction.
    facts: Vec<Vec<F>>,
}

fn index_two<T>(items: &mut [T], first_index: usize, second_index: usize) -> (&mut T, &T) {
    let split_at_index = first_index.max(second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &second_slice[0])
    } else {
        (&mut second_slice[0], &first_slice[second_index])
    }
}

impl<F: Lattice> FactBase<F> {
    pub fn new<I: Instruction>(graph: &Graph<I>) -> FactBase<F> {
        let facts: Vec<Vec<F>> = graph
            .sub_graphs
            .iter()
            .map(|sub_graph| sub_graph.nodes().iter().map(|_| F::bottom()).collect())
            .collect();
        FactBase { facts }
    }

    pub fn get(&self, label: Label) -> Option<&F> {
        self.facts
            .get(label.sub_graph())
            .and_then(|v| v.get(label.index()))
    }

    pub fn get_mut(&mut self, label: Label) -> Option<&mut F> {
        self.facts
            .get_mut(label.sub_graph())
            .and_then(|v| v.get_mut(label.index()))
    }

    pub fn get_disjoint(&mut self, first: Label, second: Label) -> Option<(&mut F, &F)> {
        if first.sub_graph() >= self.facts.len() || second.sub_graph() >= self.facts.len() {
            return None;
        }
        if first.sub_graph() == second.sub_graph() {
            let sub_graph_facts = &mut self.facts[first.sub_graph()];
            if first.index() >= sub_graph_facts.len() || second.index() >= sub_graph_facts.len() {
                return None;
            }
            return Some(index_two(sub_graph_facts, first.index(), second.index()));
        } else {
            let (left, right) = index_two(&mut self.facts, first.sub_graph(), second.sub_graph());
            if first.index() >= left.len() || second.index() >= right.len() {
                return None;
            }
            return Some((&mut left[first.index()], &right[second.index()]));
        }
    }
}
