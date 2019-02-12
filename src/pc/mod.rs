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

type FactBase<F> = FnvHashMap<Label, RefCell<F>>;

pub fn forward_analyze<A, I, F>(analysis: &mut A, graph: &Graph<I>) -> FactBase<F>
where
    A: Analysis<I, F>,
    I: Instruction,
    F: Lattice,
{
    let mut fact_base = FnvHashMap::default();
    fact_base.insert(ENTRY, RefCell::new(F::top()));

    let mut working_set = FnvHashSet::default();
    working_set.insert(ENTRY);

    let mut pc = ENTRY;
    while let Some(new_pc) = working_set.iter().next() {
        pc = *new_pc;

        'path: loop {
            working_set.remove(&pc);
            let instruction = graph.get_instruction(pc);
            let fact = fact_base
                .get(&pc)
                .expect("we should always have a fact for our current pc")
                .borrow();

            match analysis.analyze(graph, pc, &instruction, &fact) {
                Rewrite::NoChange => {
                    drop(fact);
                    let successors = instruction.successors();

                    let mut need_new_pc = !successors.fallthrough;

                    if successors.fallthrough {
                        let left = graph.next_pc(pc);
                        fact_base
                            .entry(left)
                            .or_insert_with(|| RefCell::new(F::bottom()));

                        let old_fact = fact_base.get(&left).expect("we just inserted a new entry");

                        let fact = fact_base
                            .get(&pc)
                            .expect("we should always have a fact for our current pc");

                        // As you can see, like down below, we are relying on the idea that
                        // an instruction is never going to loop back directly on itself,
                        // so that the old & new facts are disjoint.
                        if old_fact.borrow_mut().join(&fact.borrow(), left) {
                            pc = left;
                        } else {
                            need_new_pc = true;
                        }
                    }

                    for successor in successors.jumps {
                        fact_base
                            .entry(successor)
                            .or_insert_with(|| RefCell::new(F::bottom()));

                        let old_fact = fact_base
                            .get(&successor)
                            .expect("we just inserted a new entry");

                        let fact = fact_base
                            .get(&pc)
                            .expect("we should always have a fact for our current pc");

                        if old_fact.borrow_mut().join(&fact.borrow(), successor) && successor != pc
                        {
                            working_set.insert(successor);
                        }
                    }

                    if need_new_pc {
                        break 'path;
                    }
                }
                Rewrite::Fact(new_fact) => {
                    drop(fact);
                    let successors = instruction.successors();
                    let mut need_new_pc = !successors.fallthrough;

                    if successors.fallthrough {
                        let left = graph.next_pc(pc);

                        let old_fact = fact_base
                            .entry(left)
                            .or_insert_with(|| RefCell::new(F::bottom()));
                        if old_fact.get_mut().join(&new_fact, left) {
                            pc = left;
                        } else {
                            need_new_pc = true;
                        }
                    }

                    for successor in successors.jumps {
                        let old_fact = fact_base
                            .entry(successor)
                            .or_insert_with(|| RefCell::new(F::bottom()));
                        if old_fact.get_mut().join(&new_fact, successor) && successor != pc {
                            working_set.insert(successor);
                        }
                    }

                    if need_new_pc {
                        break 'path;
                    }
                }
                Rewrite::Single(new_instruction) => panic!("not implemented yet"),
                Rewrite::Many(new_instructions) => panic!("not implemented yet"),
            }
        }
    }

    fact_base
}
