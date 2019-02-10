mod dataflow;
mod pc;

#[cfg(test)]
mod test {
    use crate::dataflow::dominator;
    use crate::dataflow::*;
    use fnv::FnvHashMap;
    use std::collections::HashMap;

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct Var(u16);

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct Constant(usize);

    #[derive(Copy, Clone, Debug, Hash)]
    enum Arith {
        Add,
        Sub,
        And,
        Or,
    }

    #[derive(Copy, Clone, Debug, Hash)]
    enum Cond {
        Eq,
        Neq,
        Lt,
        Lte,
    }

    #[derive(Copy, Clone, Debug, Hash)]
    enum RiscEntry {
        Label(Label),
    }

    #[derive(Copy, Clone, Debug, Hash)]
    enum RiscInstruction {
        Load(Var, Constant),
        Arith(Arith, Var, Var, Var),
    }

    #[derive(Copy, Clone, Debug, Hash)]
    enum RiscExit {
        Cond(Cond, Var, Var, Label, Label),
        Jump(Label),
        Ret,
    }

    impl Entry for RiscEntry {
        fn label(&self) -> Label {
            match self {
                RiscEntry::Label(l) => *l,
            }
        }
    }

    impl Instruction for RiscInstruction {}

    impl Exit for RiscExit {
        fn successors(&self) -> Vec<Label> {
            match self {
                RiscExit::Cond(_, _, _, l1, l2) => vec![*l1, *l2],
                RiscExit::Jump(l) => vec![*l],
                RiscExit::Ret => vec![],
            }
        }
    }

    #[derive(Clone)]
    struct RiscLanguage;

    impl Language for RiscLanguage {
        type Entry = RiscEntry;
        type Instruction = RiscInstruction;
        type Exit = RiscExit;
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    enum WithTop<T> {
        Top,
        Elem(T),
    }

    #[derive(Clone, Debug)]
    struct ConstFact {
        vars: HashMap<Var, WithTop<Constant>>,
    }

    impl ConstFact {
        fn new() -> ConstFact {
            ConstFact {
                vars: HashMap::new(),
            }
        }

        fn set(&mut self, var: Var, constant: Constant) {
            self.vars.insert(var, WithTop::Elem(constant));
        }

        fn get(&self, var: Var) -> Option<WithTop<Constant>> {
            self.vars.get(&var).cloned()
        }

        fn get_const(&self, var: Var) -> Option<Constant> {
            match self.vars.get(&var) {
                Some(WithTop::Elem(c)) => Some(*c),
                _ => None,
            }
        }
    }

    impl Lattice for ConstFact {
        fn bottom() -> Self {
            ConstFact::new()
        }

        fn join(&mut self, other: &Self, _label: Label) -> bool {
            let mut changed = false;
            for (k, v) in &other.vars {
                let old = self.get(*k);
                let new = match (old, v) {
                    (Some(WithTop::Top), _) => WithTop::Top,
                    (_, WithTop::Top) => WithTop::Top,
                    (Some(WithTop::Elem(x)), WithTop::Elem(y)) => {
                        if x == *y {
                            WithTop::Elem(x)
                        } else {
                            WithTop::Top
                        }
                    }
                    (None, anything) => *anything,
                };

                if old != Some(new) {
                    changed = true;
                    self.vars.insert(*k, new);
                }
            }

            changed
        }
    }

    struct ConstantPropagation;

    impl ForwardAnalysis<RiscLanguage, ConstFact> for ConstantPropagation {
        fn analyze_entry(
            &mut self,
            _graph: &Graph<RiscLanguage>,
            _label: Label,
            _entry: &RiscEntry,
            fact: ConstFact,
        ) -> ConstFact {
            fact
        }

        fn analyze_instruction(
            &mut self,
            _graph: &Graph<RiscLanguage>,
            label: Label,
            instruction: &RiscInstruction,
            analyze: AnalyzeInstruction<ConstFact>,
        ) -> Option<RewriteInstruction<RiscLanguage>> {
            match instruction {
                RiscInstruction::Load(var, constant) => {
                    analyze.fact_mut().set(*var, *constant);
                    None
                }
                RiscInstruction::Arith(arith, dst, src1, src2) => {
                    let facts = analyze.fact();

                    if let (Some(Constant(c1)), Some(Constant(c2))) =
                        (facts.get_const(*src1), facts.get_const(*src2))
                    {
                        let result = match arith {
                            Arith::Add => c1 + c2,
                            Arith::Sub => c1 - c2,
                            Arith::And => c1 & c2,
                            Arith::Or => c1 | c2,
                        };

                        return Some(analyze.replace(RiscInstruction::Load(*dst, Constant(result))));
                    }
                    None
                }
            }
        }

        fn analyze_exit(
            &mut self,
            _graph: &Graph<RiscLanguage>,
            label: Label,
            exit: &RiscExit,
            fact: &ConstFact,
        ) -> RewriteExit<RiscLanguage, ConstFact> {
            let mut facts = FnvHashMap::default();

            match exit {
                RiscExit::Cond(_cond, _src1, _src2, l1, l2) => {
                    facts.insert(*l1, fact.clone());
                    facts.insert(*l2, fact.clone());

                    RewriteExit::Done(facts)
                }
                RiscExit::Jump(l1) => {
                    facts.insert(*l1, fact.clone());

                    RewriteExit::Done(facts)
                }
                RiscExit::Ret => RewriteExit::Done(facts),
            }
        }
    }

    #[test]
    fn constant_test() {
        let entry = Label(0);
        let loop_body = Label(1);
        let exit = Label(2);

        let block0 = BasicBlock::new(
            RiscEntry::Label(entry),
            vec![
                RiscInstruction::Load(Var(0), Constant(0)),
                RiscInstruction::Load(Var(1), Constant(1)),
                RiscInstruction::Load(Var(2), Constant(5)),
            ],
            RiscExit::Jump(loop_body),
        );

        let block1 = BasicBlock::new(
            RiscEntry::Label(loop_body),
            vec![RiscInstruction::Arith(Arith::Sub, Var(2), Var(2), Var(1))],
            RiscExit::Cond(Cond::Eq, Var(2), Var(0), exit, loop_body),
        );

        let block2 = BasicBlock::new(RiscEntry::Label(exit), vec![], RiscExit::Ret);

        let graph = Graph::from_blocks(vec![block0, block1, block2]);

        let mut analysis = ConstantPropagation;
        // Strictly speaking, we'd want an entry fact that had all vars as Top..
        let fact_base = forward_analysis(&mut analysis, &graph, entry, ConstFact::bottom());
        println!("{:?}", fact_base);
    }

    #[test]
    fn dominator_test() {
        let block1: BasicBlock<RiscLanguage> =
            BasicBlock::new(RiscEntry::Label(Label(1)), vec![], RiscExit::Jump(Label(2)));

        let block2: BasicBlock<RiscLanguage> = BasicBlock::new(
            RiscEntry::Label(Label(2)),
            vec![],
            RiscExit::Cond(Cond::Eq, Var(0), Var(1), Label(3), Label(4)),
        );

        let block3: BasicBlock<RiscLanguage> =
            BasicBlock::new(RiscEntry::Label(Label(3)), vec![], RiscExit::Jump(Label(5)));

        let block4: BasicBlock<RiscLanguage> =
            BasicBlock::new(RiscEntry::Label(Label(4)), vec![], RiscExit::Jump(Label(5)));

        let block5: BasicBlock<RiscLanguage> =
            BasicBlock::new(RiscEntry::Label(Label(5)), vec![], RiscExit::Jump(Label(2)));
        let graph = Graph::from_blocks(vec![block1, block2, block3, block4, block5]);

        let mut dom_analysis = dominator::DominatorAnalysis;
        let dominators = forward_analysis(
            &mut dom_analysis,
            &graph,
            Label(1),
            dominator::DominatorFact::bottom(),
        );

        println!("dominators {{");
        for (label, dom) in dominators {
            println!("\t{:?}: {:?}", label, dom.dominates);
        }
        println!("}}");
    }
}

fn main() {
    println!("Hello, world!");
}
