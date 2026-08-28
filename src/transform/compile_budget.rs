use super::error::TransformError;
use super::limits::TransformLimits;

/// Scalar admission ledger for direction compilation. It deliberately does
/// not own profile-derived data; the borrowed plans are admitted before any
/// decoded curve storage is materialized.
#[derive(Debug)]
pub(super) struct CompileBudget {
    limits: TransformLimits,
    curve_entries: usize,
    clut_entries: usize,
    pending_bytes: usize,
    actual_bytes: usize,
    temporary_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BudgetCheckpoint {
    curve_entries: usize,
    clut_entries: usize,
    pending_bytes: usize,
    actual_bytes: usize,
    temporary_bytes: usize,
}

impl CompileBudget {
    pub(super) const fn new(limits: TransformLimits) -> Self {
        Self {
            limits,
            curve_entries: 0,
            clut_entries: 0,
            pending_bytes: 0,
            actual_bytes: 0,
            temporary_bytes: 0,
        }
    }

    pub(super) fn checkpoint(&self) -> BudgetCheckpoint {
        BudgetCheckpoint {
            curve_entries: self.curve_entries,
            clut_entries: self.clut_entries,
            pending_bytes: self.pending_bytes,
            actual_bytes: self.actual_bytes,
            temporary_bytes: self.temporary_bytes,
        }
    }

    #[cfg(test)]
    pub(super) fn test_state(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.curve_entries,
            self.clut_entries,
            self.pending_bytes,
            self.actual_bytes,
            self.temporary_bytes,
        )
    }

    pub(super) fn rollback(&mut self, checkpoint: BudgetCheckpoint) {
        self.curve_entries = checkpoint.curve_entries;
        self.clut_entries = checkpoint.clut_entries;
        self.pending_bytes = checkpoint.pending_bytes;
        self.actual_bytes = checkpoint.actual_bytes;
        self.temporary_bytes = checkpoint.temporary_bytes;
    }

    pub(super) fn admit_curve(
        &mut self,
        entries: usize,
        decoded_bytes: usize,
    ) -> Result<(), TransformError> {
        let next_entries = self
            .curve_entries
            .checked_add(entries)
            .ok_or(TransformError::ResourceLimit("compiled curve entries"))?;
        if next_entries > self.limits.max_curve_entries {
            return Err(TransformError::ResourceLimit("compiled curve entries"));
        }
        let next_pending = self
            .pending_bytes
            .checked_add(decoded_bytes)
            .ok_or(TransformError::ResourceLimit("compiled transform bytes"))?;
        self.check_total_values(next_pending, self.actual_bytes, self.temporary_bytes)?;
        self.curve_entries = next_entries;
        self.pending_bytes = next_pending;
        Ok(())
    }

    pub(super) fn admit_matrix_storage(
        &mut self,
        channels: usize,
        owned_headers: usize,
    ) -> Result<(), TransformError> {
        let outer = channels
            .checked_mul(std::mem::size_of::<super::curve::Curve>())
            .ok_or(TransformError::ResourceLimit("compiled transform bytes"))?;
        let next_pending = self
            .pending_bytes
            .checked_add(
                outer
                    .checked_add(owned_headers)
                    .ok_or(TransformError::ResourceLimit("compiled transform bytes"))?,
            )
            .ok_or(TransformError::ResourceLimit("compiled transform bytes"))?;
        self.check_total_values(next_pending, self.actual_bytes, self.temporary_bytes)?;
        self.pending_bytes = next_pending;
        Ok(())
    }

    pub(super) fn admit_storage(
        &mut self,
        bytes: usize,
        label: &'static str,
    ) -> Result<(), TransformError> {
        let next_pending = self
            .pending_bytes
            .checked_add(bytes)
            .ok_or(TransformError::ResourceLimit(label))?;
        self.check_total_values(next_pending, self.actual_bytes, self.temporary_bytes)?;
        self.pending_bytes = next_pending;
        Ok(())
    }

    pub(super) fn admit_clut(&mut self, entries: usize) -> Result<(), TransformError> {
        let next_entries = self
            .clut_entries
            .checked_add(entries)
            .ok_or(TransformError::ResourceLimit("compiled CLUT entries"))?;
        if next_entries > self.limits.max_clut_entries {
            return Err(TransformError::ResourceLimit("compiled CLUT entries"));
        }
        self.clut_entries = next_entries;
        Ok(())
    }

    pub(super) fn try_new_vec<T>(
        &mut self,
        count: usize,
        planned_bytes: usize,
        label: &'static str,
    ) -> Result<Vec<T>, TransformError> {
        let requested_bytes = count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(TransformError::ResourceLimit(label))?;
        if requested_bytes != planned_bytes {
            return Err(TransformError::ResourceLimit(label));
        }
        self.try_candidate(
            planned_bytes,
            label,
            || {
                let mut candidate = Vec::new();
                candidate
                    .try_reserve_exact(count)
                    .map_err(|_| TransformError::ResourceLimit(label))?;
                Ok(candidate)
            },
            |candidate: &Vec<T>| {
                candidate
                    .capacity()
                    .checked_mul(std::mem::size_of::<T>())
                    .ok_or(TransformError::ResourceLimit(label))
            },
        )
    }

    pub(super) fn try_candidate<T, F, M>(
        &mut self,
        planned_bytes: usize,
        label: &'static str,
        make: F,
        measure: M,
    ) -> Result<T, TransformError>
    where
        F: FnOnce() -> Result<T, TransformError>,
        M: FnOnce(&T) -> Result<usize, TransformError>,
    {
        let checkpoint = self.checkpoint();
        if planned_bytes > self.pending_bytes {
            return Err(TransformError::ResourceLimit(label));
        }
        let candidate = match make() {
            Ok(candidate) => candidate,
            Err(error) => {
                self.rollback(checkpoint);
                return Err(error);
            }
        };
        let actual_bytes = match measure(&candidate) {
            Ok(bytes) => bytes,
            Err(error) => {
                drop(candidate);
                self.rollback(checkpoint);
                return Err(error);
            }
        };
        self.pending_bytes -= planned_bytes;
        self.actual_bytes = match self.actual_bytes.checked_add(actual_bytes) {
            Some(value) => value,
            None => {
                drop(candidate);
                self.rollback(checkpoint);
                return Err(TransformError::ResourceLimit(label));
            }
        };
        if let Err(error) = self.check_total() {
            drop(candidate);
            self.rollback(checkpoint);
            return Err(error);
        }
        Ok(candidate)
    }

    pub(super) fn commit_owned(
        &mut self,
        planned_bytes: usize,
        actual_bytes: usize,
        label: &'static str,
    ) -> Result<(), TransformError> {
        let checkpoint = self.checkpoint();
        if planned_bytes > self.pending_bytes {
            return Err(TransformError::ResourceLimit(label));
        }
        self.pending_bytes -= planned_bytes;
        self.actual_bytes = match self.actual_bytes.checked_add(actual_bytes) {
            Some(value) => value,
            None => {
                self.rollback(checkpoint);
                return Err(TransformError::ResourceLimit(label));
            }
        };
        if let Err(error) = self.check_total() {
            self.rollback(checkpoint);
            return Err(error);
        }
        Ok(())
    }

    fn check_total(&self) -> Result<(), TransformError> {
        self.check_total_values(self.pending_bytes, self.actual_bytes, self.temporary_bytes)
    }

    fn check_total_values(
        &self,
        pending_bytes: usize,
        actual_bytes: usize,
        temporary_bytes: usize,
    ) -> Result<(), TransformError> {
        let total = pending_bytes
            .checked_add(actual_bytes)
            .and_then(|value| value.checked_add(temporary_bytes))
            .ok_or(TransformError::ResourceLimit("compiled transform bytes"))?;
        if total > self.limits.max_compiled_bytes {
            return Err(TransformError::ResourceLimit("compiled transform bytes"));
        }
        Ok(())
    }
}
