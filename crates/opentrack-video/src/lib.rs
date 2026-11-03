//! Bounded FFmpeg-backed video decoding for the Project video window.
//!
//! The crate isolates the FFmpeg ABI in one place. With the `ffmpeg` feature it
//! decodes the video frame nearest a timestamp to RGBA; without the feature (or
//! without FFmpeg installed at build time) it reports that the backend is
//! unavailable instead of pretending to play video.

use std::path::Path;

/// One decoded frame, ready to upload as a texture.
#[derive(Clone, Debug, PartialEq)]
pub struct RgbaFrame {
    pub width: u32,
    pub height: u32,
    /// Tightly packed RGBA8 rows.
    pub rgba: Vec<u8>,
    /// Presentation time of the frame in seconds.
    pub timestamp_seconds: f64,
}

/// Whether this build includes the FFmpeg decoder.
pub const fn backend_available() -> bool {
    cfg!(feature = "ffmpeg")
}

#[cfg(feature = "ffmpeg")]
mod backend {
    use super::RgbaFrame;
    use ffmpeg::software::scaling;
    use ffmpeg::util::frame::video::Video as VideoFrame;
    use ffmpeg_next as ffmpeg;
    use ffmpeg_next::format;
    use ffmpeg_next::media::Type;
    use ffmpeg_next::util::format::pixel::Pixel;
    use std::path::Path;

    /// Most decoded frames per lookup, so a long file cannot stall the UI.
    const MAX_DECODED_FRAMES: usize = 4_000;

    pub struct VideoDecoder {
        input: format::context::Input,
        decoder: ffmpeg::decoder::Video,
        scaler: scaling::Context,
        stream_index: usize,
        time_base: f64,
        duration_seconds: Option<f64>,
    }

    impl VideoDecoder {
        pub fn open(path: &Path) -> Result<Self, String> {
            ffmpeg::init().map_err(|error| format!("FFmpeg init failed: {error}"))?;
            let input =
                format::input(path).map_err(|error| format!("Could not open video: {error}"))?;
            let stream = input
                .streams()
                .best(Type::Video)
                .ok_or_else(|| "the file has no video stream".to_owned())?;
            let stream_index = stream.index();
            let time_base = f64::from(stream.time_base());
            let stream_duration = stream.duration();
            let duration_seconds = if stream_duration > 0 {
                Some(stream_duration as f64 * time_base)
            } else {
                let total = input.duration();
                (total > 0).then(|| total as f64 / 1_000_000.0)
            };
            let context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
                .map_err(|error| format!("Could not read the video codec: {error}"))?;
            let decoder = context
                .decoder()
                .video()
                .map_err(|error| format!("Unsupported video codec: {error}"))?;
            let scaler = scaling::Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                Pixel::RGBA,
                decoder.width(),
                decoder.height(),
                scaling::Flags::BILINEAR,
            )
            .map_err(|error| format!("Could not create a video scaler: {error}"))?;
            Ok(Self {
                input,
                decoder,
                scaler,
                stream_index,
                time_base,
                duration_seconds,
            })
        }

        pub fn duration_seconds(&self) -> Option<f64> {
            self.duration_seconds
        }

        /// Decodes the frame nearest `seconds`, seeking to about a second before.
        pub fn frame_at(&mut self, seconds: f64) -> Result<RgbaFrame, String> {
            if !seconds.is_finite() || seconds < 0.0 {
                return Err("the video position is not a valid time".to_owned());
            }
            let target = seconds;
            let seek_to = (target - 1.0).max(0.0);
            if seek_to > 0.0 {
                let offset = (seek_to * 1_000_000.0) as i64;
                // A failed seek is not fatal: decoding from the start still
                // produces a correct (if slower) answer.
                let _ = self.input.seek(offset, ..);
            }
            self.decoder.flush();
            let mut decoded = 0usize;
            let mut best: Option<RgbaFrame> = None;
            let mut frame = VideoFrame::empty();
            'outer: for (stream, packet) in self.input.packets() {
                if stream.index() != self.stream_index {
                    continue;
                }
                self.decoder
                    .send_packet(&packet)
                    .map_err(|error| format!("Could not decode the video: {error}"))?;
                while self.decoder.receive_frame(&mut frame).is_ok() {
                    decoded += 1;
                    let timestamp = frame
                        .pts()
                        .map(|pts| pts as f64 * self.time_base)
                        .unwrap_or(0.0);
                    let candidate = convert_frame(&mut self.scaler, &frame, timestamp)?;
                    let replace = match &best {
                        Some(current) => {
                            (timestamp - target).abs() < (current.timestamp_seconds - target).abs()
                        }
                        None => true,
                    };
                    if replace {
                        best = Some(candidate);
                    }
                    if timestamp >= target || decoded >= MAX_DECODED_FRAMES {
                        break 'outer;
                    }
                }
            }
            best.ok_or_else(|| "no frame could be decoded from the video".to_owned())
        }
    }

    fn convert_frame(
        scaler: &mut scaling::Context,
        frame: &VideoFrame,
        timestamp_seconds: f64,
    ) -> Result<RgbaFrame, String> {
        let mut rgba = VideoFrame::empty();
        scaler
            .run(frame, &mut rgba)
            .map_err(|error| format!("Could not convert the video frame: {error}"))?;
        let width = rgba.width();
        let height = rgba.height();
        let stride = rgba.stride(0);
        let row_bytes = width as usize * 4;
        let mut data = Vec::with_capacity(row_bytes * height as usize);
        for row in 0..height as usize {
            let start = row * stride;
            if start + row_bytes > rgba.data(0).len() {
                return Err("the converted frame is shorter than expected".to_owned());
            }
            data.extend_from_slice(&rgba.data(0)[start..start + row_bytes]);
        }
        Ok(RgbaFrame {
            width,
            height,
            rgba: data,
            timestamp_seconds,
        })
    }
}

#[cfg(feature = "ffmpeg")]
pub use backend::VideoDecoder;

#[cfg(not(feature = "ffmpeg"))]
pub struct VideoDecoder;

#[cfg(not(feature = "ffmpeg"))]
impl VideoDecoder {
    pub fn open(_path: &Path) -> Result<Self, String> {
        Err("this build has no FFmpeg video decoder".to_owned())
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        None
    }

    pub fn frame_at(&mut self, _seconds: f64) -> Result<RgbaFrame, String> {
        Err("this build has no FFmpeg video decoder".to_owned())
    }
}

#[cfg(all(test, feature = "ffmpeg"))]
mod tests {
    use super::*;

    /// Writes a tiny YUV4MPEG2 clip: two 2x2 frames at 25 fps, pure red.
    fn write_y4m(path: &Path) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\n");
        for _ in 0..2 {
            bytes.extend_from_slice(b"FRAME\n");
            // Y = 76, U = 85, V = 255 is saturated red in BT.601.
            bytes.extend_from_slice(&[76, 76, 76, 76]);
            bytes.push(85);
            bytes.push(255);
        }
        std::fs::write(path, bytes).expect("write y4m");
    }

    #[test]
    fn frames_decode_to_rgba_with_timestamps() {
        let root = std::env::temp_dir().join(format!("opentrack-video-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let path = root.join("clip.y4m");
        write_y4m(&path);

        assert!(backend_available());
        let mut decoder = VideoDecoder::open(&path).expect("open");
        let first = decoder.frame_at(0.0).expect("first frame");
        assert_eq!((first.width, first.height), (2, 2));
        assert_eq!(first.rgba.len(), 2 * 2 * 4);
        assert!(first.timestamp_seconds.abs() < 0.001);
        assert_eq!(first.rgba[3], 255, "opaque alpha");
        assert!(
            first.rgba[0] > 180 && first.rgba[1] < 60 && first.rgba[2] < 60,
            "BT.601 red: {:?}",
            &first.rgba[0..4]
        );

        let second = decoder.frame_at(0.04).expect("second frame");
        assert!((second.timestamp_seconds - 0.04).abs() < 0.001);
        assert_eq!((second.width, second.height), (2, 2));

        // Missing and invalid files are refused, not guessed.
        assert!(VideoDecoder::open(&root.join("missing.y4m")).is_err());
        assert!(decoder.frame_at(-1.0).is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(all(test, not(feature = "ffmpeg")))]
mod tests {
    use super::*;

    #[test]
    fn the_backend_reports_when_it_is_not_built() {
        assert!(!backend_available());
        assert!(VideoDecoder::open(Path::new("clip.mp4")).is_err());
    }
}
