use codeguard_cli::guard_integration::{
    reader::read_native,
    scope::{FrozenObligations, assess_scope},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};
#[path = "support/guard_integration.rs"]
mod fixture;
thread_local! {
    static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
struct AllocationProbe;
static LARGE_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for AllocationProbe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() >= 256 * 1024 && TRACK.with(|v| v.get()) {
            LARGE_ALLOCATIONS.fetch_add(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: AllocationProbe = AllocationProbe;
#[test]
fn scope_budget_rejects_repeated_large_id_before_gap_allocation() {
    let evidence = read_native(
        &serde_json::to_vec(&fixture::valid()).unwrap(),
        &fixture::invocation(),
    )
    .unwrap();
    let targets = BTreeMap::from([(
        "x".repeat(256 * 1024),
        (0..64).map(|n| format!("target-{n}")).collect(),
    )]);
    LARGE_ALLOCATIONS.store(0, Ordering::Relaxed);
    TRACK.with(|v| v.set(true));
    let frozen = FrozenObligations::new(targets);
    if let Ok(frozen) = &frozen {
        let _ = assess_scope(&evidence, frozen);
    }
    TRACK.with(|v| v.set(false));
    let bytes = LARGE_ALLOCATIONS.load(Ordering::Relaxed);
    println!(
        "large_allocation_bytes={bytes} rejected={}",
        frozen.is_err()
    );
    assert!(
        bytes <= guardengine::integration::MAX_ARTIFACT_BYTES / 2,
        "scope gap copies allocated before rejection: {bytes}"
    );
    assert_eq!(frozen.err(), Some("scope construction budget exceeded"));
}

#[test]
fn scope_admission_counts_escaping_and_keeps_small_required_scopes() {
    let targets = || (0..16).map(|n| format!("target-{n}")).collect::<Vec<_>>();
    assert!(FrozenObligations::new(BTreeMap::from([("x".repeat(64 * 1024), targets())])).is_ok());
    assert_eq!(
        FrozenObligations::new(BTreeMap::from([("\u{0001}".repeat(64 * 1024), targets())])).err(),
        Some("scope construction budget exceeded")
    );
    assert!(
        FrozenObligations::new(BTreeMap::from([(
            "x".repeat(256 * 1024),
            vec!["target".into()]
        )]))
        .is_ok()
    );
}
