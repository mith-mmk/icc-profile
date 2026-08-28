use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

pub(crate) struct Probe;

thread_local! {
    static WATCH_SIZE: Cell<usize> = const { Cell::new(0) };
    static WATCH_HITS: Cell<usize> = const { Cell::new(0) };
    static WATCH_TOTAL: Cell<usize> = const { Cell::new(0) };
    static FAIL_ONCE_SIZE: Cell<usize> = const { Cell::new(0) };
    static FAIL_HITS: Cell<usize> = const { Cell::new(0) };
    static LIVE_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
    static LIVE_PEAK: Cell<isize> = const { Cell::new(0) };
}

fn record(size: usize) {
    let _ = WATCH_SIZE.try_with(|watch| {
        if watch.get() != 0 {
            let _ = WATCH_TOTAL.try_with(|total| total.set(total.get() + size));
            if size == watch.get() {
                let _ = WATCH_HITS.try_with(|hits| hits.set(hits.get() + 1));
            }
        }
    });
}

fn live_delta(delta: isize) {
    let _ = LIVE_ACTIVE.try_with(|active| {
        if active.get() {
            let _ = LIVE_BYTES.try_with(|live| {
                let value = live.get() + delta;
                live.set(value);
                let _ = LIVE_PEAK.try_with(|peak| peak.set(peak.get().max(value)));
            });
        }
    });
}

fn should_fail(size: usize) -> bool {
    FAIL_ONCE_SIZE
        .try_with(|slot| {
            if slot.get() == size && size != 0 {
                slot.set(0);
                FAIL_HITS.set(FAIL_HITS.get() + 1);
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
}

unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        if should_fail(layout.size()) {
            return std::ptr::null_mut();
        }
        let result = unsafe { System.alloc(layout) };
        if !result.is_null() {
            live_delta(layout.size() as isize);
        }
        result
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        if should_fail(layout.size()) {
            return std::ptr::null_mut();
        }
        let result = unsafe { System.alloc_zeroed(layout) };
        if !result.is_null() {
            live_delta(layout.size() as isize);
        }
        result
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        live_delta(-(layout.size() as isize));
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        if should_fail(size) {
            return std::ptr::null_mut();
        }
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            live_delta(size as isize - layout.size() as isize);
        }
        result
    }
}

struct Restore<T: Copy + 'static> {
    slot: &'static std::thread::LocalKey<Cell<T>>,
    value: T,
}

impl<T: Copy + 'static> Drop for Restore<T> {
    fn drop(&mut self) {
        self.slot.set(self.value);
    }
}

pub(crate) fn fail_once<T>(size: usize, operation: impl FnOnce() -> T) -> T {
    struct FailRestore {
        previous_size: usize,
        previous_hits: usize,
        completed: bool,
    }

    impl Drop for FailRestore {
        fn drop(&mut self) {
            FAIL_ONCE_SIZE.set(self.previous_size);
            if !self.completed {
                FAIL_HITS.set(self.previous_hits);
            }
        }
    }

    let mut restore = FailRestore {
        previous_size: FAIL_ONCE_SIZE.get(),
        previous_hits: FAIL_HITS.get(),
        completed: false,
    };
    FAIL_HITS.set(0);
    FAIL_ONCE_SIZE.set(size);
    let result = operation();
    restore.completed = true;
    result
}

pub(crate) fn failure_hits() -> usize {
    FAIL_HITS.get()
}

pub(crate) fn live_scope<T>(operation: impl FnOnce() -> T) -> (T, isize, isize) {
    struct Guard {
        previous_active: bool,
        previous_bytes: isize,
        previous_peak: isize,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            LIVE_ACTIVE.set(self.previous_active);
            LIVE_BYTES.set(self.previous_bytes);
            LIVE_PEAK.set(self.previous_peak);
        }
    }

    let previous_active = LIVE_ACTIVE.get();
    assert!(!previous_active);
    let previous_bytes = LIVE_BYTES.get();
    let previous_peak = LIVE_PEAK.get();
    LIVE_BYTES.set(0);
    LIVE_PEAK.set(0);
    LIVE_ACTIVE.set(true);
    let guard = Guard {
        previous_active,
        previous_bytes,
        previous_peak,
    };
    let result = operation();
    let live = LIVE_BYTES.get();
    let peak = LIVE_PEAK.get();
    drop(guard);
    (result, live, peak)
}

pub(crate) fn watch<T>(size: usize, operation: impl FnOnce() -> T) -> (T, usize, usize) {
    let previous_size = WATCH_SIZE.get();
    let previous_hits = WATCH_HITS.get();
    let previous_total = WATCH_TOTAL.get();
    let _restore_size = Restore {
        slot: &WATCH_SIZE,
        value: previous_size,
    };
    let _restore_hits = Restore {
        slot: &WATCH_HITS,
        value: previous_hits,
    };
    let _restore_total = Restore {
        slot: &WATCH_TOTAL,
        value: previous_total,
    };
    WATCH_HITS.set(0);
    WATCH_TOTAL.set(0);
    WATCH_SIZE.set(size);
    let result = operation();
    (result, WATCH_HITS.get(), WATCH_TOTAL.get())
}
