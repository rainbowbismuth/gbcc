use fnv::{FnvHashMap, FnvHashSet};
use std::cell::RefCell;

#[cfg(test)]
mod test;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Label {
    sub_graph: u32,
    index: u32,
}

impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "L.{:04x}.{:04x}", self.sub_graph, self.index)
    }
}

impl Label {
    pub fn new(sub_graph: u32, index: u32) -> Label {
        Label { sub_graph, index }
    }

    fn sub_graph(&self) -> usize {
        self.sub_graph as usize
    }

    fn index(&self) -> usize {
        self.index as usize
    }
}

#[derive(Clone, Debug)]
pub struct Successors {
    pub fallthrough: bool,
    pub jumps: Vec<Label>,
}

impl Successors {
    pub fn fallthrough() -> Successors {
        Successors {
            fallthrough: true,
            jumps: vec![],
        }
    }

    pub fn conditional(jumps: Vec<Label>) -> Successors {
        Successors {
            fallthrough: true,
            jumps,
        }
    }

    pub fn halt() -> Successors {
        Successors {
            fallthrough: false,
            jumps: vec![],
        }
    }

    pub fn goto(jumps: Vec<Label>) -> Successors {
        Successors {
            fallthrough: false,
            jumps,
        }
    }
}

// TODO: We might want some sort of... optional context passed in?
pub trait Instruction: Clone {
    fn successors(&self) -> Successors;
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node<I: Instruction> {
    Instruction(I),
    SubGraph(u32),
}

impl<I: Instruction> Node<I> {
    fn new(instruction: I) -> Self {
        Node::Instruction(instruction)
    }
}

pub struct Graph<I: Instruction> {
    sub_graphs: Vec<SubGraph<I>>,
}

const ENTRY: Label = Label {
    sub_graph: 0,
    index: 0,
};

const LABEL_FORWARD_LIMIT: usize = 10_000;

impl<I: Instruction> Graph<I> {
    fn new(code: Vec<I>) -> Self {
        let nodes = code.into_iter().map(Node::new).collect();
        Graph {
            sub_graphs: vec![SubGraph::new(0, ENTRY, nodes)],
        }
    }

    fn start(&self) -> Label {
        self.forward_label_completely(ENTRY)
    }

    fn backward_label(&self, label: Label) -> Label {
        self.sub_graphs[label.sub_graph()].entry
    }

    fn forward_label_completely(&self, mut label: Label) -> Label {
        for _ in 0..LABEL_FORWARD_LIMIT {
            let (new_label, cont) = self.forward_label(label);
            if !cont {
                return new_label;
            }
            label = new_label;
        }

        panic!("sub graph loop detected");
    }

    fn forward_label(&self, label: Label) -> (Label, bool) {
        match self.get_node(label) {
            Node::Instruction(i) => (label, false),
            Node::SubGraph(sub_graph) => (Label::new(*sub_graph, 0), true),
        }
    }

    fn next_pc(&self, mut label: Label) -> Label {
        loop {
            label.index += 1;
            label = self.forward_label_completely(label);

            if self.node_exists(label) {
                return label;
            } else {
                if label.sub_graph == 0 {
                    panic!("We should have never reached the end of the program without encountering a halt or return like instruction")
                }
                // We went off the end of this sub-graph, so we implicitly return to it's entry point.
                label = self.backward_label(label);
            }
        }
    }

    fn get_node(&self, label: Label) -> &Node<I> {
        &self.sub_graphs[label.sub_graph()].nodes[label.index()]
    }

    fn get_instruction(&self, label: Label) -> &I {
        match self.get_node(self.forward_label_completely(label)) {
            Node::Instruction(i) => i,
            Node::SubGraph(_) => panic!("can't happen because of forwarding"),
        }
    }

    fn node_exists(&self, label: Label) -> bool {
        self.sub_graphs[label.sub_graph()].contains(&label)
    }
}

struct SubGraph<I: Instruction> {
    number: u32,
    entry: Label,
    nodes: Vec<Node<I>>,
}

impl<I: Instruction> SubGraph<I> {
    fn new(number: u32, entry: Label, nodes: Vec<Node<I>>) -> Self {
        SubGraph {
            number,
            entry,
            nodes,
        }
    }

    fn contains(&self, label: &Label) -> bool {
        (self.number as usize) == label.sub_graph() && label.index() < self.nodes.len()
    }
}

pub trait Lattice: Sized + Clone {
    // This constructs the bottom-most fact, which is used to initialize labels we don't
    //   have any information for
    fn bottom() -> Self;

    // Used as the initial fact at the entry point
    fn top() -> Self;

    // Mutably update this current fact with the information in the 'other' fact. The label
    //   is passed for potential debugging purposes.
    //
    // This function returns a bool of whether or not you were actually changed.
    //   If you were to reach your top-most fact, this would always return false.
    fn join(&mut self, other: &Self, label: Label) -> bool;
}

// Not handling the FactBase case yet for jumps...
pub enum Rewrite<I, F> {
    NoChange,
    Fact(F),
    Single(I),
    Many(Vec<I>),
}

pub trait Analysis<I, F>
where
    I: Instruction,
    F: Lattice,
{
    fn analyze(
        &mut self,
        graph: &Graph<I>,
        label: Label,
        instruction: &I,
        fact: &F,
    ) -> Rewrite<I, F>;
}

#[derive(Clone, Debug)]
pub struct FactBase<F> {
    // Like a Graph, there's a sub_graph vector and then a fact per instruction.
    facts: Vec<Vec<F>>,
}

fn mut_two<T>(items: &mut [T], first_index: usize, second_index: usize) -> (&mut T, &T) {
    let split_at_index = first_index.max(second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

impl<F: Lattice> FactBase<F> {
    pub fn new<I: Instruction>(graph: &Graph<I>) -> FactBase<F> {
        let facts: Vec<Vec<F>> = graph
            .sub_graphs
            .iter()
            .map(|sub_graph| sub_graph.nodes.iter().map(|_| F::bottom()).collect())
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
        if first.sub_graph == second.sub_graph {
            let sub_graph_facts = &mut self.facts[first.sub_graph()];
            if first.index() >= sub_graph_facts.len() || second.index() >= sub_graph_facts.len() {
                return None;
            }
            return Some(mut_two(sub_graph_facts, first.index(), second.index()));
        } else {
            let (left, right) = mut_two(&mut self.facts, first.sub_graph(), second.sub_graph());
            if first.index() >= left.len() || second.index() >= right.len() {
                return None;
            }
            return Some((&mut left[first.index()], &right[second.index()]));
        }
    }
}

pub fn forward_analyze<A, I, F>(analysis: &mut A, graph: &Graph<I>) -> FactBase<F>
where
    A: Analysis<I, F>,
    I: Instruction,
    F: Lattice,
{
    let mut fact_base = FactBase::new(graph);
    *fact_base.get_mut(ENTRY).unwrap() = F::top();

    let mut working_set = FnvHashSet::default();
    working_set.insert(ENTRY);

    let mut pc = ENTRY;
    while let Some(new_pc) = working_set.iter().next() {
        pc = *new_pc;
        'path: loop {
            working_set.remove(&pc);
            let instruction = graph.get_instruction(pc);

            let successors = instruction.successors();
            let mut need_new_pc = !successors.fallthrough;
            let fallthrough_pc = graph.next_pc(pc);
            let (fallthrough_fact, current_fact) =
                fact_base.get_disjoint(fallthrough_pc, pc).unwrap();

            match analysis.analyze(graph, pc, &instruction, &current_fact) {
                Rewrite::NoChange => {
                    if successors.fallthrough {
                        if fallthrough_fact.join(&current_fact, fallthrough_pc) {
                            pc = fallthrough_pc;
                        } else {
                            need_new_pc = true;
                        }
                    }

                    for successor in successors.jumps {
                        let (successor_fact, current_fact) =
                            fact_base.get_disjoint(successor, pc).unwrap();

                        if successor_fact.join(&current_fact, successor) && successor != pc {
                            working_set.insert(successor);
                        }
                    }

                    if need_new_pc {
                        break 'path;
                    }
                }
                Rewrite::Fact(new_fact) => {
                    if successors.fallthrough {
                        if fallthrough_fact.join(&new_fact, fallthrough_pc) {
                            pc = fallthrough_pc;
                        } else {
                            need_new_pc = true;
                        }
                    }
                    for successor in successors.jumps {
                        let successor_fact = fact_base.get_mut(successor).unwrap();

                        if successor_fact.join(&new_fact, successor) && successor != pc {
                            working_set.insert(successor);
                        }
                    }

                    if need_new_pc {
                        break 'path;
                    }
                }
                // TODO: Notes for when I implement these, of course we're going to have to be
                //  working off of a duplicated graph. But we'll also have to update the fact base
                //  to be able to hold facts for the sub graph
                Rewrite::Single(new_instruction) => panic!("not implemented yet"),
                Rewrite::Many(new_instructions) => panic!("not implemented yet"),
            }
        }
    }

    fact_base
}
