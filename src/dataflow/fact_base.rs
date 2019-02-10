use super::graph::Label;
use fnv::FnvHashMap;

pub type FactBase<F> = FnvHashMap<Label, F>;
