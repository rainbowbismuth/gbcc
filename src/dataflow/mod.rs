pub mod dominator;
mod fact_base;
mod forward_analysis;
mod graph;
mod lattice;

pub use fact_base::FactBase;
pub use forward_analysis::{
    forward_analysis, AnalyzeInstruction, ForwardAnalysis, RewriteExit, RewriteInstruction,
};
pub use graph::{BasicBlock, Entry, Exit, Graph, Instruction, Label, Language};
pub use lattice::Lattice;
