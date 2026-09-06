//! A module for replaying input data.

use std::io::Read;

use flate2::read::ZlibDecoder;

/// A frame of input data.
pub struct InputFrame {
    /// Whether the up button is pressed.
    pub up: bool,
    /// Whether the right button is pressed.
    pub right: bool,
    /// Whether the down button is pressed.
    pub down: bool,
    /// Whether the left button is pressed.
    pub left: bool,
    /// Whether the reset button is pressed.
    pub reset: bool,
}

/// A replay of input data.
pub struct Replay {
    /// A timeline of frames where the up button is pressed.
    up: Vec<u32>,
    /// A timeline of frames where the right button is pressed.
    right: Vec<u32>,
    /// A timeline of frames where the down button is pressed.
    down: Vec<u32>,
    /// A timeline of frames where the left button is pressed.
    left: Vec<u32>,
    /// A timeline of frames where the reset button is pressed.
    reset: Vec<u32>,
}

impl Replay {
    /// Deserialize a replay from a byte slice.
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        let mut decoder = ZlibDecoder::new(data);
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded).ok()?;

        let mut offset = 0;

        let up = read_timeline(&decoded, &mut offset)?;
        let right = read_timeline(&decoded, &mut offset)?;
        let down = read_timeline(&decoded, &mut offset)?;
        let left = read_timeline(&decoded, &mut offset)?;
        let reset = read_timeline(&decoded, &mut offset)?;

        Some(Self {
            up,
            right,
            down,
            left,
            reset,
        })
    }

    /// Get the input frame for a given frame number.
    pub fn get_frame(&self, frame: u32) -> InputFrame {
        InputFrame {
            up: state_at(frame, &self.up),
            right: state_at(frame, &self.right),
            down: state_at(frame, &self.down),
            left: state_at(frame, &self.left),
            reset: state_at(frame, &self.reset),
        }
    }
}

/// Get the state of a button at a given frame number.
fn state_at(frame: u32, changes: &[u32]) -> bool {
    let mut lo = 0;
    let mut hi = changes.len();

    while lo < hi {
        let mid = (lo + hi) / 2;

        if changes[mid] <= frame {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    (lo & 1) != 0
}

/// Read a timeline of frames from a byte slice.
fn read_timeline(data: &[u8], offset: &mut usize) -> Option<Vec<u32>> {
    let count = read_u24(data, *offset)?;
    *offset += 3;

    let mut frames = Vec::with_capacity(count as usize);

    let mut current = 0u32;

    for _ in 0..count {
        let delta = read_u24(data, *offset)?;
        *offset += 3;

        current += delta;

        frames.push(current);
    }

    Some(frames)
}

/// Read a 24-bit unsigned integer from a byte slice.
fn read_u24(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 3 > data.len() {
        return None;
    }

    Some(data[offset] as u32 | ((data[offset + 1] as u32) << 8) | ((data[offset + 2] as u32) << 16))
}
