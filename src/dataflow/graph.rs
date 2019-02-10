use fnv::{FnvHashMap, FnvHashSet};

use std::fmt;
use std::ops::Index;

// A label is an unsigned integer, used to identify a block.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "L{}", self.0)
    }
}

pub trait Entry: Clone {
    fn label(&self) -> Label;
}

pub trait Instruction: Clone {}

pub trait Exit: Clone {
    fn successors(&self) -> Vec<Label>;
}

pub trait Language: Clone {
    type Entry: Entry;
    type Instruction: Instruction;
    type Exit: Exit;
}

#[derive(Clone)]
pub struct BasicBlock<L: Language> {
    pub entry: L::Entry,
    pub code: Vec<L::Instruction>,
    pub exit: L::Exit,
}

impl<L: Language> BasicBlock<L> {
    pub fn new(entry: L::Entry, code: Vec<L::Instruction>, exit: L::Exit) -> BasicBlock<L> {
        BasicBlock { entry, code, exit }
    }

    pub fn label(&self) -> Label {
        self.entry.label()
    }

    pub fn successors(&self) -> Vec<Label> {
        self.exit.successors()
    }
}

#[derive(Clone)]
pub struct Graph<L: Language> {
    blocks: FnvHashMap<Label, BasicBlock<L>>,
}

impl<L: Language> Graph<L> {
    pub fn from_blocks(blocks: Vec<BasicBlock<L>>) -> Graph<L> {
        // TODO: Error if multiple blocks w/ same label
        let mut map = FnvHashMap::default();
        for block in blocks {
            map.insert(block.label(), block);
        }
        Graph { blocks: map }
    }

    pub fn post_order_traversal(&self, entry: Label) -> Vec<Label> {
        fn go<L: Language>(
            graph: &Graph<L>,
            output: &mut Vec<Label>,
            visited: &mut FnvHashSet<Label>,
            label: Label,
        ) {
            if !visited.insert(label) {
                return;
            }
            for successor in graph.blocks[&label].successors() {
                go(graph, output, visited, successor);
            }
            output.push(label);
        }

        let mut output = vec![];
        let mut visited = FnvHashSet::default();
        go(self, &mut output, &mut visited, entry);
        output
    }

    pub fn contains(&self, label: Label) -> bool {
        self.blocks.contains_key(&label)
    }
}

impl<L: Language> Index<Label> for Graph<L> {
    type Output = BasicBlock<L>;

    fn index(&self, label: Label) -> &Self::Output {
        &self.blocks[&label]
    }
}
