use fnv::FnvHashSet;

mod analysis;
pub mod dominator;
mod fact_base;
mod graph;
mod instruction;
mod lattice;

#[cfg(test)]
mod test;

pub use analysis::*;
pub use fact_base::*;
pub use graph::*;
pub use instruction::*;
pub use lattice::*;
