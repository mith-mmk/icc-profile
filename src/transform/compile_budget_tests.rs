use super::compile_budget::CompileBudget;
use super::limits::TransformLimits;

fn limits(bytes: usize) -> TransformLimits {
    TransformLimits::builder()
        .max_compiled_bytes(bytes)
        .max_curve_entries(16)
        .build()
        .unwrap()
}

#[test]
fn fresh_vec_reconciles_actual_capacity_once() {
    let mut budget = CompileBudget::new(limits(32));
    budget.admit_curve(0, 16).unwrap();
    let candidate = budget.try_new_vec::<u8>(16, 16, "candidate").unwrap();
    assert_eq!(candidate.capacity(), 16);
}

#[test]
fn overcapacity_candidate_drops_and_restores_pending_budget() {
    let mut budget = CompileBudget::new(limits(15));
    budget.admit_curve(0, 8).unwrap();
    let checkpoint = budget.checkpoint();
    let result = budget.try_candidate(
        8,
        "candidate",
        || Ok(Vec::<u8>::with_capacity(16)),
        |candidate| Ok(candidate.capacity()),
    );
    assert!(result.is_err());
    assert_eq!(budget.checkpoint(), checkpoint);
    let retry = budget.try_new_vec::<u8>(8, 8, "candidate");
    assert!(retry.is_ok());
}

#[test]
fn failed_candidate_maker_restores_budget_for_retry() {
    let mut budget = CompileBudget::new(limits(16));
    budget.admit_curve(0, 8).unwrap();
    let checkpoint = budget.checkpoint();
    let result = budget.try_candidate::<Vec<u8>, _, _>(
        8,
        "candidate",
        || {
            Err(super::error::TransformError::ResourceLimit(
                "injected allocation",
            ))
        },
        |candidate| Ok(candidate.capacity()),
    );
    assert!(result.is_err());
    assert_eq!(budget.checkpoint(), checkpoint);
    assert!(budget.try_new_vec::<u8>(8, 8, "candidate").is_ok());
}

#[test]
fn whole_admission_is_atomic_on_entry_or_byte_failure() {
    let mut budget = CompileBudget::new(limits(8));
    let checkpoint = budget.checkpoint();
    assert!(budget.admit_curve(3, 16).is_err());
    assert_eq!(budget.checkpoint(), checkpoint);

    let mut budget = CompileBudget::new(limits(64));
    let checkpoint = budget.checkpoint();
    assert!(budget.admit_curve(17, 0).is_err());
    assert_eq!(budget.checkpoint(), checkpoint);
}

#[test]
fn fresh_vec_rejects_planned_size_mismatch_before_reserve() {
    let mut budget = CompileBudget::new(limits(8));
    budget.admit_curve(0, 8).unwrap();
    let checkpoint = budget.checkpoint();
    assert!(budget.try_new_vec::<u8>(16, 8, "candidate").is_err());
    assert_eq!(budget.checkpoint(), checkpoint);
    assert!(budget.try_new_vec::<u8>(8, 8, "candidate").is_ok());
}
