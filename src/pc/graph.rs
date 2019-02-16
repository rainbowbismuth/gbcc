use super::*;
use fnv::FnvHashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Node<I: Instruction> {
    Instruction(I),
    SubGraph(u32),
}

impl<I: Instruction> Node<I> {
    fn new(instruction: I) -> Self {
        Node::Instruction(instruction)
    }
}

pub struct Graph<I: Instruction> {
    pub(crate) sub_graphs: Vec<SubGraph<I>>,
}

pub const ENTRY: Label = Label {
    sub_graph: 0,
    index: 0,
};

const LABEL_FORWARD_LIMIT: usize = 10_000;

impl<I: Instruction> Graph<I> {
    pub fn new(code: Vec<I>) -> Self {
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

    pub(crate) fn next_pc(&self, mut label: Label) -> Label {
        loop {
            label.index += 1;
            label = self.forward_label_completely(label);

            if self.node_exists(label) {
                return label;
            } else {
                if label.sub_graph() == 0 {
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

    pub fn get_instruction(&self, label: Label) -> &I {
        match self.get_node(self.forward_label_completely(label)) {
            Node::Instruction(i) => i,
            Node::SubGraph(_) => panic!("can't happen because of forwarding"),
        }
    }

    fn node_exists(&self, label: Label) -> bool {
        self.sub_graphs[label.sub_graph()].contains(label)
    }

    pub fn post_order_traversal(&self, start: Label) -> Vec<Label> {
        let mut seen = FnvHashSet::default();
        let mut output = vec![];
        fn go<I: Instruction>(
            label: Label,
            graph: &Graph<I>,
            seen: &mut FnvHashSet<Label>,
            output: &mut Vec<Label>,
        ) {
            let forwarded = graph.forward_label_completely(label);
            if !seen.insert(forwarded) {
                return;
            }
            let successors = graph.get_instruction(forwarded).successors();
            if successors.fallthrough {
                go(graph.next_pc(forwarded), graph, seen, output);
            }
            for successor in successors.jumps {
                go(successor, graph, seen, output);
            }
            output.push(label);
        };
        go(start, self, &mut seen, &mut output);
        output
    }
}

pub(crate) struct SubGraph<I: Instruction> {
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

    pub(crate) fn nodes(&self) -> &[Node<I>] {
        &self.nodes
    }

    pub(crate) fn contains(&self, label: Label) -> bool {
        (self.number as usize) == label.sub_graph() && label.index() < self.nodes.len()
    }
}
