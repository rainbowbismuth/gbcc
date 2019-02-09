use fnv::{FnvHashMap, FnvHashSet};

use std::ops::Index;

// A label is an unsigned integer, used to identify a block.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Label(pub u32);



pub trait Instruction: Clone {
    // If this instruction begins a block, this gives the associated label
    fn label(&self) -> Option<Label>;

    // If this instruction ends a block, this gives each place we can jump to
    //   we might return Some(vec![]) if the instruction is a 'return' type instruction
    //   that is, it is the end of the block, but we're not jumping somewhere else in
    //   the current graph we are examining.
    fn successors(&self) -> Option<Vec<Label>>;
}

#[derive(Debug, Clone)]
pub struct Block<I> {
    label: Label,
    successors: Vec<Label>,
    code: Vec<I>,
}

impl<I: Instruction> Block<I> {
    pub fn new(code: Vec<I>) -> Option<Block<I>> {
        let label = code.first().and_then(|i| i.label())?;
        let successors = code.last().and_then(|i| i.successors())?;
        Some(Block { label, successors, code })
    }

    pub fn label(&self) -> Label {
        self.label
    }

    pub fn successors(&self) -> &[Label] {
        &self.successors
    }

    pub fn code(&self) -> &[I] {
        &self.code
    }
}


#[derive(Debug, Clone)]
pub struct Graph<I> {
    blocks: FnvHashMap<Label, Block<I>>,
}

impl<I: Instruction> Graph<I> {
    pub fn from_blocks(blocks: Vec<Block<I>>) -> Graph<I> {
        // TODO: Error if multiple blocks w/ same label
        let mut map = FnvHashMap::default();
        for block in blocks {
            map.insert(block.label(), block);
        }
        Graph { blocks: map }
    }

    pub fn post_order_traversal(&self, entry: Label) -> Vec<Label> {
        fn go<I: Instruction>(graph: &Graph<I>, output: &mut Vec<Label>, visited: &mut FnvHashSet<Label>, label: Label) {
            if !visited.insert(label) {
                return;
            }
            for successor in graph.blocks[&label].successors() {
                go(graph, output, visited, *successor);
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

impl<I> Index<Label> for Graph<I> {
    type Output = Block<I>;

    fn index(&self, label: Label) -> &Self::Output {
        &self.blocks[&label]
    }
}