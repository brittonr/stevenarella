// Copyright 2026
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-APACHE> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.

use crate::gl;
use std::path::{Component, Path, PathBuf};

pub const BLAKE3_HEX_LENGTH: usize = 64;
pub const RGBA_BYTES_PER_PIXEL: usize = 4;
pub const DEFAULT_MAX_WIDTH_PX: u32 = 7_680;
pub const DEFAULT_MAX_HEIGHT_PX: u32 = 4_320;
pub const DEFAULT_MIN_RECORDING_FPS: u16 = 1;
pub const DEFAULT_MAX_RECORDING_FPS: u16 = 60;
pub const DEFAULT_MAX_RECORDING_FRAMES: u32 = 600;
pub const DEFAULT_MAX_RECORDING_MILLIS: u64 = 10_000;
pub const DEFAULT_MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;
pub const DEFAULT_INLINE_RESPONSE_BYTES: u64 = 512 * 1024;

const FORMAT_PNG: &str = "png";
const FRAMEBUFFER_READ_ORIGIN_X: i32 = 0;
const FRAMEBUFFER_READ_ORIGIN_Y: i32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Screenshot,
    LatestFrame,
    Recording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFormat {
    Png,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionState {
    NotReviewed,
    Reviewed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blake3DigestHex(String);

impl Blake3DigestHex {
    pub fn new(value: impl Into<String>) -> Result<Self, CaptureValidationError> {
        let value = value.into();
        if !is_blake3_hex(&value) {
            return Err(CaptureValidationError::InvalidBlake3Digest {
                actual_len: value.len(),
            });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOutput {
    Inline,
    Artifact { relative_path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordingBounds {
    pub frame_rate_hz: u16,
    pub max_frames: Option<u32>,
    pub max_duration_millis: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureRequest {
    pub mode: CaptureMode,
    pub format: CaptureFormat,
    pub output: CaptureOutput,
    pub includes_ui: bool,
    pub recording: Option<RecordingBounds>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePolicy {
    pub capture_dir: Option<PathBuf>,
    pub max_width_px: u32,
    pub max_height_px: u32,
    pub min_recording_fps: u16,
    pub max_recording_fps: u16,
    pub max_recording_frames: u32,
    pub max_recording_millis: u64,
    pub max_artifact_bytes: u64,
    pub inline_response_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePlan {
    pub mode: CaptureMode,
    pub format: CaptureFormat,
    pub output: CaptureOutput,
    pub includes_ui: bool,
    pub artifact_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureArtifactMetadata {
    pub relative_path: PathBuf,
    pub format: CaptureFormat,
    pub width_px: u32,
    pub height_px: u32,
    pub frame_id: u64,
    pub byte_len: u64,
    pub blake3_digest: Blake3DigestHex,
    pub includes_ui: bool,
    pub redaction: RedactionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRgbaFrame {
    pub width_px: u32,
    pub height_px: u32,
    pub frame_id: u64,
    pub rgba_top_left: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureReadbackError {
    InvalidDimensions { width_px: u32, height_px: u32 },
    BufferSizeOverflow { width_px: u32, height_px: u32 },
    BufferLengthMismatch { expected: usize, actual: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureValidationError {
    UnsupportedFormat(String),
    MissingCaptureDir,
    EmptyArtifactPath,
    ArtifactPathEscapes { relative_path: PathBuf },
    RecordingBoundsUnexpected,
    RecordingBoundsRequired,
    RecordingFrameRateOutOfRange { requested: u16, min: u16, max: u16 },
    RecordingDurationRequired,
    RecordingFramesOutOfRange { requested: u32, max: u32 },
    RecordingMillisOutOfRange { requested: u64, max: u64 },
    WidthOutOfRange { requested: u32, max: u32 },
    HeightOutOfRange { requested: u32, max: u32 },
    ArtifactTooLarge { requested: u64, max: u64 },
    InvalidBlake3Digest { actual_len: usize },
}

impl CaptureFormat {
    pub fn from_name(name: &str) -> Result<Self, CaptureValidationError> {
        if name.eq_ignore_ascii_case(FORMAT_PNG) {
            return Ok(Self::Png);
        }
        Err(CaptureValidationError::UnsupportedFormat(name.to_owned()))
    }

    pub fn as_extension(self) -> &'static str {
        match self {
            Self::Png => FORMAT_PNG,
        }
    }
}

impl CapturePolicy {
    pub fn local(capture_dir: impl Into<PathBuf>) -> Self {
        Self {
            capture_dir: Some(capture_dir.into()),
            max_width_px: DEFAULT_MAX_WIDTH_PX,
            max_height_px: DEFAULT_MAX_HEIGHT_PX,
            min_recording_fps: DEFAULT_MIN_RECORDING_FPS,
            max_recording_fps: DEFAULT_MAX_RECORDING_FPS,
            max_recording_frames: DEFAULT_MAX_RECORDING_FRAMES,
            max_recording_millis: DEFAULT_MAX_RECORDING_MILLIS,
            max_artifact_bytes: DEFAULT_MAX_ARTIFACT_BYTES,
            inline_response_bytes: DEFAULT_INLINE_RESPONSE_BYTES,
        }
    }
}

pub fn validate_capture_request(
    request: &CaptureRequest,
    policy: &CapturePolicy,
) -> Result<CapturePlan, CaptureValidationError> {
    let artifact_path = match &request.output {
        CaptureOutput::Inline => None,
        CaptureOutput::Artifact { relative_path } => {
            Some(contained_artifact_path(policy, relative_path)?)
        }
    };

    match (request.mode, request.recording) {
        (CaptureMode::Recording, Some(recording)) => validate_recording_bounds(recording, policy)?,
        (CaptureMode::Recording, None) => {
            return Err(CaptureValidationError::RecordingBoundsRequired)
        }
        (_, Some(_)) => return Err(CaptureValidationError::RecordingBoundsUnexpected),
        (_, None) => {}
    }

    Ok(CapturePlan {
        mode: request.mode,
        format: request.format,
        output: request.output.clone(),
        includes_ui: request.includes_ui,
        artifact_path,
    })
}

pub fn validate_artifact_metadata(
    metadata: &CaptureArtifactMetadata,
    policy: &CapturePolicy,
) -> Result<PathBuf, CaptureValidationError> {
    validate_dimensions(metadata.width_px, metadata.height_px, policy)?;
    validate_artifact_size(metadata.byte_len, policy)?;
    contained_artifact_path(policy, &metadata.relative_path)
}

pub fn validate_dimensions(
    width_px: u32,
    height_px: u32,
    policy: &CapturePolicy,
) -> Result<(), CaptureValidationError> {
    if width_px == 0 || width_px > policy.max_width_px {
        return Err(CaptureValidationError::WidthOutOfRange {
            requested: width_px,
            max: policy.max_width_px,
        });
    }
    if height_px == 0 || height_px > policy.max_height_px {
        return Err(CaptureValidationError::HeightOutOfRange {
            requested: height_px,
            max: policy.max_height_px,
        });
    }
    Ok(())
}

pub fn read_current_framebuffer_rgba_top_left(
    width_px: u32,
    height_px: u32,
    frame_id: u64,
) -> Result<CapturedRgbaFrame, CaptureReadbackError> {
    let expected_len = rgba_buffer_len(width_px, height_px)?;
    let mut rgba_bottom_left = vec![0; expected_len];
    gl::read_pixels_rgba(
        FRAMEBUFFER_READ_ORIGIN_X,
        FRAMEBUFFER_READ_ORIGIN_Y,
        width_px,
        height_px,
        &mut rgba_bottom_left,
    );
    captured_rgba_from_bottom_left(width_px, height_px, frame_id, &rgba_bottom_left)
}

pub fn captured_rgba_from_bottom_left(
    width_px: u32,
    height_px: u32,
    frame_id: u64,
    rgba_bottom_left: &[u8],
) -> Result<CapturedRgbaFrame, CaptureReadbackError> {
    Ok(CapturedRgbaFrame {
        width_px,
        height_px,
        frame_id,
        rgba_top_left: normalize_rgba_bottom_left_to_top_left(
            width_px,
            height_px,
            rgba_bottom_left,
        )?,
    })
}

pub fn normalize_rgba_bottom_left_to_top_left(
    width_px: u32,
    height_px: u32,
    rgba_bottom_left: &[u8],
) -> Result<Vec<u8>, CaptureReadbackError> {
    let expected_len = rgba_buffer_len(width_px, height_px)?;
    if rgba_bottom_left.len() != expected_len {
        return Err(CaptureReadbackError::BufferLengthMismatch {
            expected: expected_len,
            actual: rgba_bottom_left.len(),
        });
    }

    let row_stride = rgba_row_stride_bytes(width_px, height_px)?;
    let mut rgba_top_left = vec![0; expected_len];
    for top_row in 0..height_px as usize {
        let bottom_row = (height_px as usize) - top_row - 1;
        let source_start = bottom_row * row_stride;
        let target_start = top_row * row_stride;
        let source_end = source_start + row_stride;
        let target_end = target_start + row_stride;
        rgba_top_left[target_start..target_end]
            .copy_from_slice(&rgba_bottom_left[source_start..source_end]);
    }

    Ok(rgba_top_left)
}

pub fn rgba_buffer_len(width_px: u32, height_px: u32) -> Result<usize, CaptureReadbackError> {
    validate_readback_dimensions(width_px, height_px)?;
    let row_stride = rgba_row_stride_bytes(width_px, height_px)?;
    row_stride
        .checked_mul(height_px as usize)
        .ok_or(CaptureReadbackError::BufferSizeOverflow {
            width_px,
            height_px,
        })
}

fn validate_readback_dimensions(width_px: u32, height_px: u32) -> Result<(), CaptureReadbackError> {
    if width_px == 0 || height_px == 0 {
        return Err(CaptureReadbackError::InvalidDimensions {
            width_px,
            height_px,
        });
    }
    Ok(())
}

fn rgba_row_stride_bytes(width_px: u32, height_px: u32) -> Result<usize, CaptureReadbackError> {
    (width_px as usize).checked_mul(RGBA_BYTES_PER_PIXEL).ok_or(
        CaptureReadbackError::BufferSizeOverflow {
            width_px,
            height_px,
        },
    )
}

fn validate_artifact_size(
    byte_len: u64,
    policy: &CapturePolicy,
) -> Result<(), CaptureValidationError> {
    if byte_len > policy.max_artifact_bytes {
        return Err(CaptureValidationError::ArtifactTooLarge {
            requested: byte_len,
            max: policy.max_artifact_bytes,
        });
    }
    Ok(())
}

fn validate_recording_bounds(
    recording: RecordingBounds,
    policy: &CapturePolicy,
) -> Result<(), CaptureValidationError> {
    if recording.frame_rate_hz < policy.min_recording_fps
        || recording.frame_rate_hz > policy.max_recording_fps
    {
        return Err(CaptureValidationError::RecordingFrameRateOutOfRange {
            requested: recording.frame_rate_hz,
            min: policy.min_recording_fps,
            max: policy.max_recording_fps,
        });
    }

    if recording.max_frames.is_none() && recording.max_duration_millis.is_none() {
        return Err(CaptureValidationError::RecordingDurationRequired);
    }

    if let Some(max_frames) = recording.max_frames {
        if max_frames == 0 || max_frames > policy.max_recording_frames {
            return Err(CaptureValidationError::RecordingFramesOutOfRange {
                requested: max_frames,
                max: policy.max_recording_frames,
            });
        }
    }

    if let Some(max_duration_millis) = recording.max_duration_millis {
        if max_duration_millis == 0 || max_duration_millis > policy.max_recording_millis {
            return Err(CaptureValidationError::RecordingMillisOutOfRange {
                requested: max_duration_millis,
                max: policy.max_recording_millis,
            });
        }
    }

    Ok(())
}

fn contained_artifact_path(
    policy: &CapturePolicy,
    relative_path: &Path,
) -> Result<PathBuf, CaptureValidationError> {
    let capture_dir = policy
        .capture_dir
        .as_ref()
        .ok_or(CaptureValidationError::MissingCaptureDir)?;
    validate_relative_artifact_path(relative_path)?;
    Ok(capture_dir.join(relative_path))
}

fn validate_relative_artifact_path(relative_path: &Path) -> Result<(), CaptureValidationError> {
    let mut components_seen = false;
    for component in relative_path.components() {
        components_seen = true;
        match component {
            Component::Normal(name) if !name.is_empty() => {}
            _ => {
                return Err(CaptureValidationError::ArtifactPathEscapes {
                    relative_path: relative_path.to_path_buf(),
                })
            }
        }
    }
    if !components_seen {
        return Err(CaptureValidationError::EmptyArtifactPath);
    }
    Ok(())
}

fn is_blake3_hex(digest: &str) -> bool {
    digest.len() == BLAKE3_HEX_LENGTH && digest.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CAPTURE_DIR: &str = "capture-root";
    const TEST_ARTIFACT_PATH: &str = "screens/frame-0001.png";
    const TEST_WIDTH_PX: u32 = 1_920;
    const TEST_HEIGHT_PX: u32 = 1_080;
    const TEST_FRAME_ID: u64 = 42;
    const TEST_BYTE_LEN: u64 = 4_096;
    const TEST_RECORDING_FPS: u16 = 30;
    const TEST_RECORDING_FRAMES: u32 = 10;
    const TEST_BLAKE3: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const TEST_READBACK_WIDTH_PX: u32 = 2;
    const TEST_READBACK_HEIGHT_PX: u32 = 2;
    const TEST_OPAQUE_ALPHA: u8 = u8::MAX;
    const TEST_BOTTOM_LEFT_PIXEL: [u8; RGBA_BYTES_PER_PIXEL] = [10, 20, 30, TEST_OPAQUE_ALPHA];
    const TEST_BOTTOM_RIGHT_PIXEL: [u8; RGBA_BYTES_PER_PIXEL] = [40, 50, 60, TEST_OPAQUE_ALPHA];
    const TEST_TOP_LEFT_PIXEL: [u8; RGBA_BYTES_PER_PIXEL] = [70, 80, 90, TEST_OPAQUE_ALPHA];
    const TEST_TOP_RIGHT_PIXEL: [u8; RGBA_BYTES_PER_PIXEL] = [100, 110, 120, TEST_OPAQUE_ALPHA];

    #[test]
    fn valid_screenshot_artifact_request_is_planned() {
        let policy = CapturePolicy::local(TEST_CAPTURE_DIR);
        let request = CaptureRequest {
            mode: CaptureMode::Screenshot,
            format: CaptureFormat::Png,
            output: CaptureOutput::Artifact {
                relative_path: PathBuf::from(TEST_ARTIFACT_PATH),
            },
            includes_ui: true,
            recording: None,
        };

        let plan = validate_capture_request(&request, &policy).expect("request should pass");

        assert_eq!(plan.mode, CaptureMode::Screenshot);
        assert_eq!(plan.format, CaptureFormat::Png);
        assert_eq!(
            plan.artifact_path,
            Some(PathBuf::from(TEST_CAPTURE_DIR).join(TEST_ARTIFACT_PATH))
        );
    }

    #[test]
    fn valid_recording_bounds_are_accepted() {
        let policy = CapturePolicy::local(TEST_CAPTURE_DIR);
        let request = CaptureRequest {
            mode: CaptureMode::Recording,
            format: CaptureFormat::Png,
            output: CaptureOutput::Artifact {
                relative_path: PathBuf::from(TEST_ARTIFACT_PATH),
            },
            includes_ui: true,
            recording: Some(RecordingBounds {
                frame_rate_hz: TEST_RECORDING_FPS,
                max_frames: Some(TEST_RECORDING_FRAMES),
                max_duration_millis: None,
            }),
        };

        let plan = validate_capture_request(&request, &policy).expect("recording should pass");

        assert_eq!(plan.mode, CaptureMode::Recording);
    }

    #[test]
    fn valid_artifact_metadata_is_accepted() {
        let policy = CapturePolicy::local(TEST_CAPTURE_DIR);
        let metadata = CaptureArtifactMetadata {
            relative_path: PathBuf::from(TEST_ARTIFACT_PATH),
            format: CaptureFormat::Png,
            width_px: TEST_WIDTH_PX,
            height_px: TEST_HEIGHT_PX,
            frame_id: TEST_FRAME_ID,
            byte_len: TEST_BYTE_LEN,
            blake3_digest: Blake3DigestHex::new(TEST_BLAKE3).expect("digest should pass"),
            includes_ui: true,
            redaction: RedactionState::NotReviewed,
        };

        let path = validate_artifact_metadata(&metadata, &policy).expect("metadata should pass");

        assert_eq!(
            path,
            PathBuf::from(TEST_CAPTURE_DIR).join(TEST_ARTIFACT_PATH)
        );
    }

    #[test]
    fn rgba_readback_normalizes_gl_bottom_left_origin() {
        let rgba_bottom_left = [
            TEST_BOTTOM_LEFT_PIXEL,
            TEST_BOTTOM_RIGHT_PIXEL,
            TEST_TOP_LEFT_PIXEL,
            TEST_TOP_RIGHT_PIXEL,
        ]
        .concat();
        let expected_top_left = [
            TEST_TOP_LEFT_PIXEL,
            TEST_TOP_RIGHT_PIXEL,
            TEST_BOTTOM_LEFT_PIXEL,
            TEST_BOTTOM_RIGHT_PIXEL,
        ]
        .concat();

        let frame = captured_rgba_from_bottom_left(
            TEST_READBACK_WIDTH_PX,
            TEST_READBACK_HEIGHT_PX,
            TEST_FRAME_ID,
            &rgba_bottom_left,
        )
        .expect("valid RGBA readback should normalize");

        assert_eq!(frame.width_px, TEST_READBACK_WIDTH_PX);
        assert_eq!(frame.height_px, TEST_READBACK_HEIGHT_PX);
        assert_eq!(frame.frame_id, TEST_FRAME_ID);
        assert_eq!(frame.rgba_top_left, expected_top_left);
    }

    #[test]
    fn rgba_readback_rejects_wrong_buffer_length() {
        let expected = rgba_buffer_len(TEST_READBACK_WIDTH_PX, TEST_READBACK_HEIGHT_PX)
            .expect("fixture dimensions should pass");
        let actual = expected - RGBA_BYTES_PER_PIXEL;
        let short_buffer = vec![0; actual];

        let err = normalize_rgba_bottom_left_to_top_left(
            TEST_READBACK_WIDTH_PX,
            TEST_READBACK_HEIGHT_PX,
            &short_buffer,
        )
        .expect_err("short buffer rejected");

        assert_eq!(
            err,
            CaptureReadbackError::BufferLengthMismatch { expected, actual }
        );
    }

    #[test]
    fn rgba_readback_rejects_empty_dimensions() {
        let err = rgba_buffer_len(TEST_READBACK_WIDTH_PX, 0).expect_err("height rejected");

        assert_eq!(
            err,
            CaptureReadbackError::InvalidDimensions {
                width_px: TEST_READBACK_WIDTH_PX,
                height_px: 0,
            }
        );
    }

    #[test]
    fn unsupported_format_fails_closed() {
        let err = CaptureFormat::from_name("webp").expect_err("webp not supported yet");

        assert_eq!(
            err,
            CaptureValidationError::UnsupportedFormat("webp".to_owned())
        );
    }

    #[test]
    fn artifact_path_escape_is_rejected() {
        let policy = CapturePolicy::local(TEST_CAPTURE_DIR);
        let request = CaptureRequest {
            mode: CaptureMode::Screenshot,
            format: CaptureFormat::Png,
            output: CaptureOutput::Artifact {
                relative_path: PathBuf::from("../outside.png"),
            },
            includes_ui: true,
            recording: None,
        };

        let err = validate_capture_request(&request, &policy).expect_err("escape rejected");

        assert_eq!(
            err,
            CaptureValidationError::ArtifactPathEscapes {
                relative_path: PathBuf::from("../outside.png"),
            }
        );
    }

    #[test]
    fn recording_without_explicit_bounds_is_rejected() {
        let policy = CapturePolicy::local(TEST_CAPTURE_DIR);
        let request = CaptureRequest {
            mode: CaptureMode::Recording,
            format: CaptureFormat::Png,
            output: CaptureOutput::Artifact {
                relative_path: PathBuf::from(TEST_ARTIFACT_PATH),
            },
            includes_ui: true,
            recording: Some(RecordingBounds {
                frame_rate_hz: TEST_RECORDING_FPS,
                max_frames: None,
                max_duration_millis: None,
            }),
        };

        let err = validate_capture_request(&request, &policy).expect_err("unbounded rejected");

        assert_eq!(err, CaptureValidationError::RecordingDurationRequired);
    }

    #[test]
    fn invalid_digest_is_rejected() {
        let err = Blake3DigestHex::new("not-a-blake3-digest").expect_err("digest rejected");

        assert_eq!(
            err,
            CaptureValidationError::InvalidBlake3Digest {
                actual_len: "not-a-blake3-digest".len(),
            }
        );
    }

    #[test]
    fn zero_dimensions_are_rejected() {
        let policy = CapturePolicy::local(TEST_CAPTURE_DIR);

        let err = validate_dimensions(0, TEST_HEIGHT_PX, &policy).expect_err("width rejected");

        assert_eq!(
            err,
            CaptureValidationError::WidthOutOfRange {
                requested: 0,
                max: DEFAULT_MAX_WIDTH_PX,
            }
        );
    }
}
