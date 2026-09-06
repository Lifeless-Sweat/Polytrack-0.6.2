use serde::{Deserialize, Serialize};

use crate::track::Block;

/// A dataset entry, which contains a list of blocks and a list of runs.
#[derive(Serialize, Deserialize)]
pub struct Entry {
    /// The list of blocks in this track, with absolute coordinates and extra metadata.
    pub blocks: Vec<Block>,
    /// The list of runs for this track, with visited order and time.
    pub runs: Vec<Run>,
}

/// A run of a track, which contains the order of blocks visited and the time taken.
#[derive(Serialize, Deserialize)]
pub struct Run {
    /// The order of blocks visited in this run, as a list of block indices.
    pub visited_order: Vec<usize>,
    /// The time taken for this run, in milliseconds.
    pub time: u64,
}

/// A leaderboard entry, which contains the recording ID, number of frames, track ID, and position.
#[derive(Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// The recording ID of this leaderboard entry.
    pub recording: String,
    /// The number of frames taken for this leaderboard entry.
    pub frames: u64,
    /// The track ID of this leaderboard entry.
    pub track: String,
    /// The position of this leaderboard entry in the leaderboard.
    pub position: usize,
}
