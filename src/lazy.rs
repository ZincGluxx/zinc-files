// Copyright 2026 ZincGluxx
// SPDX-License-Identifier: GPL-3.0-only

/// A value initialized on first use.
///
/// The initialization closure is run at most once. Keeping this small state
/// machine separate from the compio runtime makes the "initialize exactly
/// once" behavior unit-testable without creating an io_uring runtime.
#[derive(Debug, Default)]
pub(crate) struct Lazy<T> {
    value: Option<T>,
}

impl<T> Lazy<T> {
    pub(crate) const fn new() -> Self {
        Self { value: None }
    }

    pub(crate) fn get_or_init(&mut self, init: impl FnOnce() -> T) -> &T {
        self.value.get_or_insert_with(init)
    }

    #[cfg(test)]
    fn is_initialized(&self) -> bool {
        self.value.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::Lazy;
    use std::cell::Cell;

    #[test]
    fn initializes_once_and_reuses_value() {
        let initializations = Cell::new(0);
        let mut lazy = Lazy::new();
        assert!(!lazy.is_initialized());

        assert_eq!(
            *lazy.get_or_init(|| {
                initializations.set(initializations.get() + 1);
                7
            }),
            7
        );
        assert!(lazy.is_initialized());
        assert_eq!(
            *lazy.get_or_init(|| {
                initializations.set(initializations.get() + 1);
                9
            }),
            7
        );
        assert_eq!(initializations.get(), 1);
    }
}
