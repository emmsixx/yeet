// SPDX-License-Identifier: GPL-3.0-or-later

/// Metadata about the exact staged tree that a generator saw.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryContext {
    pub branch: String,
    pub staged_tree: String,
    pub stat: String,
    pub patch: String,
    pub stat_truncation: Truncation,
    pub patch_truncation: Truncation,
}

impl RepositoryContext {
    pub fn any_truncated(&self) -> bool {
        self.stat_truncation.truncated || self.patch_truncation.truncated
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Truncation {
    pub truncated: bool,
    pub original_code_points: usize,
    pub retained_code_points: usize,
}
