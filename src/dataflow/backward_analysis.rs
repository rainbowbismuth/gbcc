use fnv::FnvHashMap;

use super::fact_base::FactBase;
use super::graph::{Graph, Label, Language};
use super::lattice::Lattice;

pub struct AnalyzeInstructionBackward<'a, F> {
    fact: &'a mut F,
}

impl<'a, F> AnalyzeInstructionBackward<'a, F> {
    fn new(fact: &'a mut F) -> Self {
        AnalyzeInstructionBackward { fact }
    }

    pub fn fact(&self) -> &F {
        self.fact
    }

    pub fn fact_mut(self) -> &'a mut F {
        self.fact
    }

    pub fn replace<L: Language>(
        self,
        instruction: L::Instruction,
    ) -> RewriteInstructionBackward<L> {
        RewriteInstructionBackward(RewriteInstructionEnum::Single(instruction))
    }

    pub fn replace_many<L: Language>(
        self,
        instructions: Vec<L::Instruction>,
    ) -> RewriteInstructionBackward<L> {
        RewriteInstructionBackward(RewriteInstructionEnum::Multiple(instructions))
    }

    pub fn replace_with_graph<L: Language>(
        self,
        exit: L::Exit,
        sub_graph: Graph<L>,
        entry: L::Entry,
    ) -> RewriteInstructionBackward<L> {
        RewriteInstructionBackward(RewriteInstructionEnum::Graph(exit, sub_graph, entry))
    }
}

pub struct AnalyzeExitBackward<'a, F> {
    fact: &'a mut F,
}

impl<'a, F> AnalyzeExitBackward<'a, F> {
    fn new(fact: &'a mut F) -> Self {
        AnalyzeExitBackward { fact }
    }

    pub fn fact(&self) -> &F {
        self.fact
    }

    pub fn fact_mut(self) -> &'a mut F {
        self.fact
    }

    pub fn replace<L: Language>(self, exit: L::Exit) -> RewriteExitBackward<L> {
        RewriteExitBackward(RewriteExitEnum::Single(exit))
    }

    pub fn replace_many<L: Language>(
        self,
        instructions: Vec<L::Instruction>,
        exit: L::Exit,
    ) -> RewriteExitBackward<L> {
        RewriteExitBackward(RewriteExitEnum::Extend(instructions, exit))
    }

    pub fn replace_with_graph<L: Language>(
        self,
        exit: L::Exit,
        sub_graph: Graph<L>,
    ) -> RewriteExitBackward<L> {
        RewriteExitBackward(RewriteExitEnum::Graph(exit, sub_graph))
    }
}

#[derive(Clone)]
pub struct RewriteInstructionBackward<L: Language>(RewriteInstructionEnum<L>);

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

#[derive(Clone)]
pub struct RewriteExitBackward<L: Language>(RewriteExitEnum<L>);

#[derive(Clone)]
enum RewriteExitEnum<L: Language> {
    // Replace the currently analyzed instruction with this single, new instruction.
    Single(L::Exit),

    // Replace this current instruction with this vector of instructions and a new exit.
    Extend(Vec<L::Instruction>, L::Exit),

    // Replace this current instruction with a sub-graph.
    //   The first argument is the 'jump' that is going to replace the current exit.
    //   The second argument is a graph with the label that the first argument instruction jumps into.
    Graph(L::Exit, Graph<L>),
}

pub trait BackwardAnalysis<L: Language, F> {
    fn analyze_exit(
        &mut self,
        graph: &Graph<L>,
        label: Label,
        exit: &L::Exit,
        analyze: AnalyzeExitBackward<F>,
    ) -> Option<RewriteExitBackward<L>>;

    fn analyze_instruction(
        &mut self,
        graph: &Graph<L>,
        label: Label,
        instruction: &L::Instruction,
        analyze: AnalyzeInstructionBackward<F>,
    ) -> Option<RewriteInstructionBackward<L>>;

    fn analyze_entry(
        &mut self,
        graph: &Graph<L>,
        label: Label,
        entry: &L::Entry,
        fact: F,
    ) -> FactBase<F>;
}

pub fn backward_analysis<L, A, F>(analysis: &mut A, graph: &Graph<L>, entry: Label) -> FactBase<F>
where
    L: Language,
    A: BackwardAnalysis<L, F>,
    F: Lattice,
{
    let mut fact_base = FnvHashMap::default();
    fixed_point_backward_graph(analysis, &graph, entry, &mut fact_base);
    fact_base
}

fn fixed_point_backward_graph<L, A, F>(
    analysis: &mut A,
    graph: &Graph<L>,
    entry: Label,
    fact_base: &mut FactBase<F>,
) where
    L: Language,
    A: BackwardAnalysis<L, F>,
    F: Lattice,
{
    let mut to_visit = graph.post_order_traversal(entry);
    to_visit.reverse();

    while let Some(label) = to_visit.pop() {
        if !graph.contains(label) {
            // We don't need to analyze any blocks outside of our sub graph.
            continue;
        }

        let output_fact_base = fixed_point_backward_block(analysis, &graph, label, fact_base);

        for predecessor in graph.direct_predecessors(label) {
            let old_fact = fact_base.entry(predecessor).or_insert_with(F::bottom);

            if !old_fact.join(&output_fact_base[&predecessor], predecessor) {
                // We didn't change so we don't need to re-examine this predecessor
                continue;
            }

            if !to_visit.contains(&predecessor) {
                to_visit.push(predecessor);
            }
        }
    }
}

fn fixed_point_backward_block<L, A, F>(
    analysis: &mut A,
    graph: &Graph<L>,
    label: Label,
    fact_base: &FactBase<F>,
) -> FactBase<F>
where
    L: Language,
    A: BackwardAnalysis<L, F>,
    F: Lattice,
{
    let mut fact = fact_base
        .get(&label)
        .expect("We should always have a fact to start from")
        .clone();
    let mut block = graph[label].clone();

    while let Some(rewrite) = analysis.analyze_exit(
        graph,
        label,
        &block.exit,
        AnalyzeExitBackward::new(&mut fact),
    ) {
        match rewrite {
            RewriteExitBackward(RewriteExitEnum::Single(exit)) => {
                block.exit = exit;
            }
            RewriteExitBackward(RewriteExitEnum::Extend(instructions, exit)) => {
                block.code.extend(instructions);
                block.exit = exit;
            }
            RewriteExitBackward(RewriteExitEnum::Graph(_exit, _sub_graph)) => {
                panic!("not implemented yet");
            }
        }
    }

    let mut counter = 0;
    while counter < block.code.len() {
        let index = block.code.len() - (counter + 1);
        match analysis.analyze_instruction(
            graph,
            label,
            &block.code[index],
            AnalyzeInstructionBackward::new(&mut fact),
        ) {
            Some(RewriteInstructionBackward(RewriteInstructionEnum::Single(inst))) => {
                block.code[index] = inst;
            }
            Some(RewriteInstructionBackward(RewriteInstructionEnum::Multiple(insts))) => {
                block.code.splice(index..index + 1, insts);
            }
            Some(RewriteInstructionBackward(RewriteInstructionEnum::Graph(
                _exit,
                _sub_graph,
                _entry,
            ))) => {
                panic!("Unimplemented");
            }
            None => {
                counter += 1;
            }
        }
    }
    analysis.analyze_entry(&graph, label, &block.entry, fact)
}
