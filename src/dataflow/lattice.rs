use super::graph::Label;

pub trait Lattice: Sized + Clone {
    // This constructs the bottom-most fact, which is used to initialize labels we don't
    //   have any information for
    fn bottom() -> Self;

    // Mutably update this current fact with the information in the 'other' fact. The label
    //   is passed for potential debugging purposes.
    //
    // This function returns a bool of whether or not you were actually changed.
    //   If you were to reach your top-most fact, this would always return false.
    fn join(&mut self, other: &Self, label: Label) -> bool;
}