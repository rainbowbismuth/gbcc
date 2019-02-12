use super::*;

// Not handling the FactBase case yet for jumps...
pub enum Rewrite<I, F> {
    NoChange,
    Fact(F),
    Single(I),
    Many(Vec<I>),
}

pub trait Analysis<I, F>
where
    I: Instruction,
    F: Lattice,
{
    fn analyze(
        &mut self,
        graph: &Graph<I>,
        label: Label,
        instruction: &I,
        fact: &F,
    ) -> Rewrite<I, F>;
}

pub fn forward_analyze<A, I, F>(analysis: &mut A, graph: &Graph<I>) -> FactBase<F>
where
    A: Analysis<I, F>,
    I: Instruction,
    F: Lattice,
{
    let mut fact_base = FactBase::new(graph);
    *fact_base.get_mut(ENTRY).unwrap() = F::top();

    let mut working_set = FnvHashSet::default();
    working_set.insert(ENTRY);

    let mut pc = ENTRY;
    while let Some(new_pc) = working_set.iter().next() {
        pc = *new_pc;
        'path: loop {
            // TODO: Could add a 'same pc' counter to detect infinite rewrite mistakes

            working_set.remove(&pc);
            let instruction = graph.get_instruction(pc);

            let successors = instruction.successors();
            let mut need_new_pc = !successors.fallthrough;
            let fallthrough_pc = graph.next_pc(pc);
            let (fallthrough_fact, current_fact) =
                fact_base.get_disjoint(fallthrough_pc, pc).unwrap();

            match analysis.analyze(graph, pc, &instruction, &current_fact) {
                Rewrite::NoChange => {
                    if successors.fallthrough {
                        if fallthrough_fact.join(&current_fact, fallthrough_pc) {
                            pc = fallthrough_pc;
                        } else {
                            need_new_pc = true;
                        }
                    }

                    for successor in successors.jumps {
                        let (successor_fact, current_fact) =
                            fact_base.get_disjoint(successor, pc).unwrap();

                        if successor_fact.join(&current_fact, successor) && successor != pc {
                            working_set.insert(successor);
                        }
                    }

                    if need_new_pc {
                        break 'path;
                    }
                }
                Rewrite::Fact(new_fact) => {
                    if successors.fallthrough {
                        if fallthrough_fact.join(&new_fact, fallthrough_pc) {
                            pc = fallthrough_pc;
                        } else {
                            need_new_pc = true;
                        }
                    }
                    for successor in successors.jumps {
                        let successor_fact = fact_base.get_mut(successor).unwrap();

                        if successor_fact.join(&new_fact, successor) && successor != pc {
                            working_set.insert(successor);
                        }
                    }

                    if need_new_pc {
                        break 'path;
                    }
                }
                // TODO: Notes for when I implement these, of course we're going to have to be
                //  working off of a duplicated graph. But we'll also have to update the fact base
                //  to be able to hold facts for the sub graph
                Rewrite::Single(new_instruction) => panic!("not implemented yet"),
                Rewrite::Many(new_instructions) => panic!("not implemented yet"),
            }
        }
    }

    fact_base
}
