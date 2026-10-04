//! Bounded operation counts for keyed bulk-selection reloads.
//!
//! This documents the current scaling limit, rather than promising a quadratic
//! algorithm forever. An indexed implementation should lower these counts and
//! update this characterization while preserving the selection assertions.
use ratagrid::{Column, GridModel};
use std::{cell::Cell, rc::Rc};

struct CountedId {
    value: usize,
    comparisons: Rc<Cell<usize>>,
}

impl PartialEq for CountedId {
    fn eq(&self, other: &Self) -> bool {
        self.comparisons.set(self.comparisons.get() + 1);
        self.value == other.value
    }
}

impl Eq for CountedId {}

fn reload_comparisons(resident: usize, marked: usize) -> usize {
    let comparisons = Rc::new(Cell::new(0));
    let identity_comparisons = Rc::clone(&comparisons);
    let mut model = GridModel::new(
        vec![Column::new("ID", 8, |value: &usize| value.to_string())],
        (0..resident).collect(),
    )
    .with_row_id(move |value| CountedId {
        value: *value,
        comparisons: Rc::clone(&identity_comparisons),
    });
    model.set_selected_indices(0..marked);
    comparisons.set(0);
    model.replace_rows((0..resident).rev().collect());

    // IDs follow the reversed replacement, independently of insertion indices.
    assert_eq!(
        model.selected_indices().collect::<Vec<_>>(),
        (resident - marked..resident).collect::<Vec<_>>()
    );
    assert_eq!(model.selected(), None);
    comparisons.get()
}

#[test]
fn keyed_bulk_reload_currently_scans_every_resident_row_for_each_mark() {
    assert_eq!(reload_comparisons(200, 50), 10_000);
    assert_eq!(reload_comparisons(200, 200), 40_000);
    assert_eq!(reload_comparisons(400, 400), 160_000);
}
