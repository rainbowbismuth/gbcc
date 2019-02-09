use fnv::FnvHashMap;

use super::lattice::Lattice;
use super::graph::{Label, Instruction, Graph};

pub struct Analyze<'a, F> {
    fact: &'a mut F
}

impl<'a, F> Analyze<'a, F> {
    fn new(fact: &'a mut F) -> Self {
        Analyze { fact }
    }

    pub fn fact(&self) -> &F {
        self.fact
    }

    pub fn fact_mut(self) -> &'a mut F {
        self.fact
    }

    pub fn replace_single<I>(self, instruction: I) -> Rewrite<I> {
        Rewrite::new(RewriteEnum::Single(instruction))
    }

    pub fn replace_multiple<I>(self, instructions: Vec<I>) -> Rewrite<I> {
        Rewrite::new(RewriteEnum::Multiple(instructions))
    }

    pub fn replace_with_graph<I>(self, jump: I, sub_graph: Graph<I>, exit: I) -> Rewrite<I> {
        Rewrite::new(RewriteEnum::Graph(jump, sub_graph, exit))
    }
}

#[derive(Debug, Clone)]
pub struct Rewrite<I> {
    rewrite: RewriteEnum<I>
}

impl<I> Rewrite<I> {
    fn new(rewrite: RewriteEnum<I>) -> Self {
        Rewrite { rewrite }
    }
}

#[derive(Debug, Clone)]
enum RewriteEnum<I> {
    // Replace the currently analyzed instruction with this single, new instruction.
    Single(I),

    // Replace this current instruction with this vector of instructions.
    //   None of the instructions should have labels or successors.
    Multiple(Vec<I>),

    // Replace this current instruction with a sub-graph.
    //   The first argument is the 'jump' that is going to replace the current instruction.
    //   The second argument is a graph with the label that the first argument instruction jumps into.
    //   The final argument is a label instruction that will head the rest of the block.
    Graph(I, Graph<I>, I),
}


pub trait ForwardAnalysis<I, F> {
    // Analyze the current instruction in the block identified by label in the given graph.
    //   If we return a rewrite, we should have not have modified the fact passed in.

    //  Not sure how to handle things like pushing different facts to different successors..
    //   for the case of conditionals..
    fn analyze(&mut self,
               graph: &Graph<I>,
               label: Label,
               instruction: &I,
               analyze: Analyze<F>)
               -> Option<Rewrite<I>>;
}


pub type FactBase<F> = FnvHashMap<Label, F>;


pub fn forward_analysis<I, A, F>(analysis: &mut A, graph: &Graph<I>, entry: Label, entry_fact: F) -> FactBase<F>
    where
        I: Instruction,
        A: ForwardAnalysis<I, F>,
        F: Lattice
{
    let mut fact_base = FnvHashMap::default();
    fact_base.insert(entry, entry_fact);

    fixed_point_forward_graph(analysis, &graph, entry, &mut fact_base);

    fact_base
}

fn fixed_point_forward_graph<I, A, F>(analysis: &mut A, graph: &Graph<I>, entry: Label, fact_base: &mut FactBase<F>)
    where
        I: Instruction,
        A: ForwardAnalysis<I, F>,
        F: Lattice
{
    let mut to_visit = graph.post_order_traversal(entry);

    while let Some(label) = to_visit.pop() {
        if !graph.contains(label) {
            // We don't need to analyze any blocks outside of our sub graph.
            continue;
        }

        let output_fact = fixed_point_forward_block(analysis, &graph, label, fact_base);

        for successor in graph[label].successors() {
            let old_fact = fact_base.entry(*successor).or_insert_with(F::bottom);

            if !old_fact.join(&output_fact, *successor) {
                // We didn't change so we don't need to re-examine this successor
                continue;
            }

            if !to_visit.contains(successor) {
                to_visit.push(*successor);
            }
        }
    }
}

fn fixed_point_forward_block<I, A, F>(analysis: &mut A, graph: &Graph<I>, label: Label, fact_base: &mut FactBase<F>) -> F
    where
        I: Instruction,
        A: ForwardAnalysis<I, F>,
        F: Lattice
{
    let mut fact = fact_base.get(&label).expect("We should always have a fact to start from").clone();
    let mut code = Vec::from(graph[label].code());
    let mut index = 0;
    while index < code.len() {
        if let Some(rewrite) = analysis.analyze(graph, label, &code[index], Analyze::new(&mut fact)) {
            match rewrite.rewrite {

                // TODO: Might want to check that we are replacing these instructions
                //  with those of the same kind! I.E. Not a label or jump when neither of those.
                RewriteEnum::Single(inst) => {
                    code[index] = inst;
                }

                RewriteEnum::Multiple(insts) => {
                    code.splice(index..index + insts.len(), insts);
                }

                RewriteEnum::Graph(_jmp, _sub_graph, _cont) => {
                    panic!("Unimplemented");
                }
            }
        } else {
            index += 1;
        }
    }
    fact
}