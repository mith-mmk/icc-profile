//! Direction and rendering-intent route selection.
//!
//! Keeping tag precedence separate from stage decoding prevents a malformed
//! selected tag from being hidden by a later fallback.

use super::profile::Profile;

pub(super) struct IntentTagSelection<'a> {
    pub(super) data: Option<&'a [u8]>,
    pub(super) signature: Option<[u8; 4]>,
    pub(super) used_fallback: bool,
}

pub(super) fn select_intent_tag<'a>(
    profile: &'a Profile,
    prefix: &[u8; 3],
    intent: u32,
) -> IntentTagSelection<'a> {
    // ICC has no A2B3/B2A3 transform tags. Absolute colorimetric uses the
    // relative colorimetric (suffix 1) route plus the media-white bridge.
    let selected = if intent == 3 { 1 } else { intent.min(2) };
    let mut signature = [0u8; 4];
    signature[..3].copy_from_slice(prefix);
    signature[3] = b'0' + selected as u8;
    if let Some(data) = profile.tag(u32::from_be_bytes(signature)) {
        return IntentTagSelection {
            data: Some(data),
            signature: Some(signature),
            used_fallback: false,
        };
    }
    if selected == 0 {
        return IntentTagSelection {
            data: None,
            signature: None,
            used_fallback: false,
        };
    }
    signature[3] = b'0';
    let data = profile.tag(u32::from_be_bytes(signature));
    IntentTagSelection {
        signature: data.map(|_| signature),
        used_fallback: data.is_some(),
        data,
    }
}

pub(super) fn find_intent_tag<'a>(
    profile: &'a Profile,
    prefix: &[u8; 3],
    intent: u32,
) -> Option<&'a [u8]> {
    select_intent_tag(profile, prefix, intent).data
}
