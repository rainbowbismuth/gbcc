mod analysis;
pub mod dominator;
mod graph;
mod lattice;

pub use analysis::{
    forward_analysis, AnalyzeInstruction, FactBase, ForwardAnalysis, RewriteExit,
    RewriteInstruction,
};
pub use graph::{BasicBlock, Entry, Exit, Graph, Instruction, Label, Language};
pub use lattice::Lattice;
