mod analysis;
mod graph;
mod lattice;

pub use analysis::{AnalyzeInstruction, RewriteInstruction, RewriteExit, FactBase, ForwardAnalysis, forward_analysis};
pub use graph::{Label, Entry, Instruction, Exit, Language, BasicBlock, Graph};
pub use lattice::Lattice;