//! Repairing recording files left behind by an abnormal exit.
//!
//! A recording writes its samples as they arrive but can only patch the RIFF
//! and `data` lengths when it finalizes. A crash, power loss or kill leaves the
//! audio on disk with a header that claims zero frames, so the file is
//! unreadable even though the samples are intact. [`repair_take`] rebuilds the
//! header from the file's real length, which is the only part that is missing.
//!
//! The repair never guesses at audio content: it reads the `fmt ` chunk that the
//! recorder wrote at creation, truncates a partial trailing frame, and rewrites
//! the two length fields. Calling it on a finalized file is a no-op.

use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

/// One take that could be made readable again.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveredTake {
    pub channels: u16,
    pub sample_rate: u32,
    pub frames: u64,
    /// True when the header was rewritten, false when the file was already valid.
    pub repaired: bool,
}

impl RecoveredTake {
    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.frames as f64 / f64::from(self.sample_rate)
    }
}

/// Describes one WAV chunk found while scanning the header.
struct Chunk {
    id: [u8; 4],
    /// Offset of the chunk's size field.
    size_offset: u64,
    /// Offset of the chunk's first payload byte.
    payload_offset: u64,
    size: u64,
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    bytes
        .get(offset..offset + 2)
        .map(|slice| u16::from_le_bytes([slice[0], slice[1]]))
}

/// Scans the chunk list of a RIFF/WAVE file without trusting its lengths.
/// Returns the declared RIFF size and every chunk up to and including `data`.
fn scan_chunks(file: &mut File, length: u64) -> Result<(u64, Vec<Chunk>), String> {
    let mut header = [0_u8; 12];
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    file.read_exact(&mut header)
        .map_err(|error| format!("the file is too short to be a WAV recording: {error}"))?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Err("the file is not a RIFF/WAVE recording".to_owned());
    }

    let riff_size = u64::from(u32::from_le_bytes([
        header[4], header[5], header[6], header[7],
    ]));
    let mut chunks = Vec::new();
    let mut offset = 12_u64;
    // The data chunk is the last one the recorder writes, so scanning stops
    // there; a truncated header cannot describe anything after it.
    while offset + 8 <= length {
        let mut head = [0_u8; 8];
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
        file.read_exact(&mut head)
            .map_err(|error| error.to_string())?;
        let mut id = [0_u8; 4];
        id.copy_from_slice(&head[0..4]);
        let size = u64::from(u32::from_le_bytes([head[4], head[5], head[6], head[7]]));
        let payload_offset = offset + 8;
        chunks.push(Chunk {
            id,
            size_offset: offset + 4,
            payload_offset,
            size,
        });
        if &id == b"data" {
            break;
        }
        // Chunks are word aligned; an odd payload is followed by one pad byte.
        offset = payload_offset.saturating_add(size).saturating_add(size % 2);
    }
    Ok((riff_size, chunks))
}

/// Makes a partially written recording readable and reports its real length.
///
/// The `fmt ` chunk must be intact, because it holds the channel count, sample
/// rate and sample width the recorder chose. Only the two length fields are
/// rewritten; no sample data is modified.
pub fn repair_take(path: &Path) -> Result<RecoveredTake, String> {
    let length = std::fs::metadata(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?
        .len();
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("could not open {}: {error}", path.display()))?;
    let (riff_size, chunks) = scan_chunks(&mut file, length)?;

    let fmt = chunks
        .iter()
        .find(|chunk| &chunk.id == b"fmt ")
        .ok_or_else(|| "the recording has no format chunk".to_owned())?;
    if fmt.size < 16 {
        return Err("the recording's format chunk is too short".to_owned());
    }
    let mut format_bytes = [0_u8; 16];
    file.seek(SeekFrom::Start(fmt.payload_offset))
        .map_err(|error| error.to_string())?;
    file.read_exact(&mut format_bytes)
        .map_err(|error| format!("the recording's format chunk is truncated: {error}"))?;
    let channels = read_u16(&format_bytes, 2).unwrap_or(0);
    let sample_rate = read_u32(&format_bytes, 4).unwrap_or(0);
    let block_align = u64::from(read_u16(&format_bytes, 12).unwrap_or(0));
    if channels == 0 || sample_rate == 0 || block_align == 0 {
        return Err("the recording's format chunk is not usable".to_owned());
    }

    let data = chunks
        .iter()
        .find(|chunk| &chunk.id == b"data")
        .ok_or_else(|| "the recording has no data chunk".to_owned())?;
    let available = length.saturating_sub(data.payload_offset);
    // A frame that was only partly flushed cannot be played; drop it.
    let data_bytes = available - available % block_align;
    let frames = data_bytes / block_align;
    let data_end = data.payload_offset + data_bytes;
    let already_valid =
        data.size == data_bytes && riff_size == data_end.saturating_sub(8) && length == data_end;

    if !already_valid {
        if length > data_end {
            file.set_len(data_end)
                .map_err(|error| format!("could not trim {}: {error}", path.display()))?;
        }
        // RIFF size counts everything after its own size field.
        file.seek(SeekFrom::Start(4))
            .map_err(|error| error.to_string())?;
        file.write_all(&((data_end.saturating_sub(8)) as u32).to_le_bytes())
            .map_err(|error| format!("could not rewrite the RIFF size: {error}"))?;
        file.seek(SeekFrom::Start(data.size_offset))
            .map_err(|error| error.to_string())?;
        file.write_all(&(data_bytes as u32).to_le_bytes())
            .map_err(|error| format!("could not rewrite the data size: {error}"))?;
        file.flush()
            .map_err(|error| format!("could not finish {}: {error}", path.display()))?;
    }

    Ok(RecoveredTake {
        channels,
        sample_rate,
        frames,
        repaired: !already_valid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("opentrack-recovery-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join("take.wav")
    }

    /// Writes a take whose header was never finalized, the state a crash leaves.
    fn write_unfinalized_take(path: &Path, frames: usize) -> Vec<f32> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        // The recorder writes to an unbuffered file, so a crash leaves exactly
        // the bytes that were flushed: header plus samples, stale lengths.
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .expect("create take");
        let mut writer = hound::WavWriter::new(file, spec).expect("create take");
        let mut expected = Vec::with_capacity(frames * 2);
        {
            for index in 0..frames {
                let left = (index as f32) / 1024.0;
                let right = -left;
                writer.write_sample(left).expect("write");
                writer.write_sample(right).expect("write");
                expected.push(left);
                expected.push(right);
            }
        }
        // Leaving the writer unfinalized reproduces the crash: the samples are on
        // disk but the RIFF and data lengths still say zero.
        std::mem::forget(writer);
        expected
    }

    #[test]
    fn repair_restores_a_header_that_was_never_finalized() {
        let path = temp_path("unfinalized");
        let expected = write_unfinalized_take(&path, 300);

        // Before repair the file reads as empty because its data size is zero.
        let before = hound::WavReader::open(&path).expect("open take");
        assert_eq!(before.len(), 0);

        let take = repair_take(&path).expect("repair");
        assert!(take.repaired);
        assert_eq!(take.channels, 2);
        assert_eq!(take.sample_rate, 48_000);
        assert_eq!(take.frames, 300);
        assert!((take.duration_seconds() - 300.0 / 48_000.0).abs() < 1e-12);

        let mut reader = hound::WavReader::open(&path).expect("reopen");
        assert_eq!(reader.len(), 600);
        let samples = reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .expect("samples");
        assert_eq!(samples, expected);
    }

    #[test]
    fn repair_of_a_finalized_take_changes_nothing() {
        let path = temp_path("finalized");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("create");
        for sample in 0..128 {
            writer.write_sample(sample as i16).expect("write");
        }
        writer.finalize().expect("finalize");
        let before = std::fs::read(&path).expect("read");

        let take = repair_take(&path).expect("repair");
        assert!(!take.repaired);
        assert_eq!(take.frames, 128);
        assert_eq!(std::fs::read(&path).expect("read"), before);
    }

    #[test]
    fn repair_drops_a_partial_trailing_frame() {
        let path = temp_path("partial-frame");
        write_unfinalized_take(&path, 64);
        // Two bytes are not a whole 8-byte stereo float frame.
        let mut file = OpenOptions::new().append(true).open(&path).expect("open");
        file.write_all(&[0, 0]).expect("append");
        drop(file);

        let take = repair_take(&path).expect("repair");
        assert_eq!(take.frames, 64);
        let header_bytes = std::fs::metadata(&path).expect("metadata").len() - 64 * 8;
        assert!(
            header_bytes < 200,
            "the partial frame is removed from the file, leaving only the header"
        );
        let reader = hound::WavReader::open(&path).expect("reopen");
        assert_eq!(reader.len(), 128);
    }

    #[test]
    fn repair_rejects_files_that_are_not_recordings() {
        let path = temp_path("not-wav");
        std::fs::write(&path, b"this is not a wave file at all").expect("write");
        assert!(repair_take(&path).is_err());

        let missing = path.with_file_name("missing.wav");
        assert!(repair_take(&missing).is_err());
    }
}
