use icc_profile::{icc, Profile, TransformError, TransformOptions};

#[test]
fn checked_namespace_preserves_type_identity_and_error_shape() {
    let _: fn(&[u8]) -> Result<Profile, TransformError> = icc::Profile::parse;
    let options: TransformOptions = icc::TransformOptions::default();
    assert!(!options.black_point_compensation);
    // Deliberately exhaustive: adding non_exhaustive or a new variant to the
    // existing error would break this downstream compatibility contract.
    let classify = |error: icc::TransformError| match error {
        TransformError::InvalidProfile(_) => 0,
        TransformError::MalformedProfile(_) => 1,
        TransformError::UnsupportedProfileFeature(_) => 2,
        TransformError::InvalidBufferLength { .. } => 3,
        TransformError::NonFiniteInput => 4,
        TransformError::ResourceLimit(_) => 5,
    };
    assert_eq!(classify(icc::Profile::parse(&[]).unwrap_err()), 0);
    let _: TransformOptions = icc::TransformOptions {
        rendering_intent: icc::RenderingIntent::RelativeColorimetric,
        black_point_compensation: false,
        clamp: false,
    };
    let _: icc::ParseLimits = icc::ParseLimits {
        max_profile_size: 1024,
        max_tag_count: 16,
        max_tag_size: 512,
        max_curve_entries: 16,
    };
    let classify_method = |method| match method {
        icc::CmykOutputBpcMethod::MidrangeStraight => 0,
        icc::CmykOutputBpcMethod::QuadraticFit => 1,
    };
    assert_eq!(classify_method(icc::CmykOutputBpcMethod::QuadraticFit), 1);
}
