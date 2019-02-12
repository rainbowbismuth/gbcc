use super::Label;

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
