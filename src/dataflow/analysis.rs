use fnv::FnvHashMap;

use super::graph::{Exit, Graph, Label, Language};
use super::lattice::Lattice;

pub struct AnalyzeInstruction<'a, F> {
    fact: &'a mut F,
}

impl<'a, F> AnalyzeInstruction<'a, F> {
    fn new(fact: &'a mut F) -> Self {
        AnalyzeInstruction { fact }
    }

    pub fn fact(&self) -> &F {
        self.fact
    }

    pub fn fact_mut(self) -> &'a mut F {
        self.fact
    }

    pub fn replace<L: Language>(self, instruction: L::Instruction) -> RewriteInstruction<L> {
        RewriteInstruction(RewriteInstructionEnum::Single(instruction))
    }

    pub fn replace_many<L: Language>(
        self,
        instructions: Vec<L::Instruction>,
    ) -> RewriteInstruction<L> {
        RewriteInstruction(RewriteInstructionEnum::Multiple(instructions))
    }

    pub fn replace_with_graph<L: Language>(
        self,
        exit: L::Exit,
        sub_graph: Graph<L>,
        entry: L::Entry,
    ) -> RewriteInstruction<L> {
        RewriteInstruction(RewriteInstructionEnum::Graph(exit, sub_graph, entry))
    }
}

#[derive(Clone)]
pub struct RewriteInstruction<L: Language>(RewriteInstructionEnum<L>);

#[derive(Clone)]
enum RewriteInstructionEnum<L: Language> {
    // Replace the currently analyzed instruction with this single, new instruction.
    Single(L::Instruction),

    // Replace this current instruction with this vector of instructions.
    Multiple(Vec<L::Instruction>),

    // Replace this current instruction with a sub-graph.
    //   The first argument is the 'jump' that is going to replace the current instruction.
    //   The second argument is a graph with the label that the first argument instruction jumps into.
    //   The final argument is a label instruction that will head the rest of the block.
    Graph(L::Exit, Graph<L>, L::Entry),
}

pub enum RewriteExit<L: Language, F> {
    // The outgoing fact bases should be for the same labels as the same instruction we just analyzed.
    Done(FactBase<F>),

    // Replace the currently analyzed instruction with this single, new instruction.
    Single(L::Exit),

    // Replace this current instruction with this vector of instructions and a new exit.
    Extend(Vec<L::Instruction>, L::Exit),

    // Replace this current instruction with a sub-graph.
    //   The first argument is the 'jump' that is going to replace the current instruction.
    //   The second argument is a graph with the label that the first argument instruction jumps into.
    //   The final argument is a label instruction that will head the rest of the block.
    Graph(L::Exit, Graph<L>, L::Entry),
}

pub trait ForwardAnalysis<L: Language, F> {
    fn analyze_entry(&mut self, graph: &Graph<L>, label: Label, entry: &L::Entry, fact: F) -> F;

    fn analyze_instruction(
        &mut self,
        graph: &Graph<L>,
        label: Label,
        instruction: &L::Instruction,
        analyze: AnalyzeInstruction<F>,
    ) -> Option<RewriteInstruction<L>>;

    fn analyze_exit(
        &mut self,
        graph: &Graph<L>,
        label: Label,
        exit: &L::Exit,
        fact: &F,
    ) -> RewriteExit<L, F>;
}

pub type FactBase<F> = FnvHashMap<Label, F>;

pub fn distribute_facts<L: Language, F: Clone>(exit: &L::Exit, fact: &F) -> FactBase<F> {
    let mut fact_base = FnvHashMap::default();
    for successor in exit.successors() {
        fact_base.insert(successor, fact.clone());
    }
    fact_base
}

pub fn forward_analysis<L, A, F>(
    analysis: &mut A,
    graph: &Graph<L>,
    entry: Label,
    entry_fact: F,
) -> FactBase<F>
where
    L: Language,
    A: ForwardAnalysis<L, F>,
    F: Lattice,
{
    let mut fact_base = FnvHashMap::default();
    fact_base.insert(entry, entry_fact);

    fixed_point_forward_graph(analysis, &graph, entry, &mut fact_base);

    fact_base
}

fn fixed_point_forward_graph<L, A, F>(
    analysis: &mut A,
    graph: &Graph<L>,
    entry: Label,
    fact_base: &mut FactBase<F>,
) where
    L: Language,
    A: ForwardAnalysis<L, F>,
    F: Lattice,
{
    let mut to_visit = graph.post_order_traversal(entry);

    while let Some(label) = to_visit.pop() {
        if !graph.contains(label) {
            // We don't need to analyze any blocks outside of our sub graph.
            continue;
        }

        let output_fact_base = fixed_point_forward_block(analysis, &graph, label, fact_base);

        for successor in graph[label].successors() {
            let old_fact = fact_base.entry(successor).or_insert_with(F::bottom);

            if !old_fact.join(&output_fact_base[&successor], successor) {
                // We didn't change so we don't need to re-examine this successor
                continue;
            }

            if !to_visit.contains(&successor) {
                to_visit.push(successor);
            }
        }
    }
}

fn fixed_point_forward_block<L, A, F>(
    analysis: &mut A,
    graph: &Graph<L>,
    label: Label,
    fact_base: &FactBase<F>,
) -> FactBase<F>
where
    L: Language,
    A: ForwardAnalysis<L, F>,
    F: Lattice,
{
    let mut fact = fact_base
        .get(&label)
        .expect("We should always have a fact to start from")
        .clone();
    let mut block = graph[label].clone();

    fact = analysis.analyze_entry(graph, label, &block.entry, fact);

    let mut index = 0;
    loop {
        while index < block.code.len() {
            match analysis.analyze_instruction(
                graph,
                label,
                &block.code[index],
                AnalyzeInstruction::new(&mut fact),
            ) {
                Some(RewriteInstruction(RewriteInstructionEnum::Single(inst))) => {
                    block.code[index] = inst;
                }
                Some(RewriteInstruction(RewriteInstructionEnum::Multiple(insts))) => {
                    block.code.splice(index..index + insts.len(), insts);
                }
                Some(RewriteInstruction(RewriteInstructionEnum::Graph(
                    _exit,
                    _sub_graph,
                    _entry,
                ))) => {
                    panic!("Unimplemented");
                }
                None => {
                    index += 1;
                }
            }
        }

        match analysis.analyze_exit(graph, label, &block.exit, &fact) {
            RewriteExit::Done(facts) => {
                return facts;
            }
            RewriteExit::Single(exit) => {
                block.exit = exit;
            }
            RewriteExit::Extend(insts, exit) => {
                block.code.extend(insts.into_iter());
                block.exit = exit;
            }
            RewriteExit::Graph(_exit, _sub_graph, _entry) => {
                panic!("Unimplemented");
            }
        }
    }
}
