#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Label {
    pub(crate) sub_graph: u32,
    pub(crate) index: u32,
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

    pub fn sub_graph(self) -> usize {
        self.sub_graph as usize
    }

    pub fn index(self) -> usize {
        self.index as usize
    }
}

// TODO: We might want some sort of... optional context passed in?
pub trait Instruction: Clone {
    fn successors(&self) -> Successors;
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
