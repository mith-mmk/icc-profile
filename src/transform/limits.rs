/// Limits applied before any allocation derived from an ICC profile.
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    pub max_profile_size: usize,
    pub max_tag_count: usize,
    pub max_tag_size: usize,
    pub max_curve_entries: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_profile_size: 64 * 1024 * 1024,
            max_tag_count: 4096,
            max_tag_size: 64 * 1024 * 1024,
            max_curve_entries: 1 << 20,
        }
    }
}
