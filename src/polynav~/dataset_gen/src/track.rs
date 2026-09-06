use anyhow::anyhow;
use polytrack_codes::{
    Block as PcBlock, Part as PcPart, Track as PcTrack, decode_track_data, v5::V5TrackMetadata,
    v6::V6TrackMetadata,
};
use polyworld::Direction;
use serde::{Deserialize, Serialize};

/// A block in a track, with absolute coordinates and extra metadata.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    /// The index of this block in the track's block list.
    pub index: usize,
    /// The ID of the part this block belongs to.
    pub part_id: u8,
    /// The absolute coordinates of this block in the track.
    pub x: i32,
    /// The absolute coordinates of this block in the track.
    pub y: i32,
    /// The absolute coordinates of this block in the track.
    pub z: i32,
    /// The rotation of this block, as a u8 (0-3).
    pub rotation: u8,
    /// The direction of this block, as a u8 (0-5).
    pub dir: u8,
    /// Whether this block is a start block.
    pub is_start: bool,
    /// The order of this block in the start sequence, if it is a start block.
    pub start_order: Option<u32>,
    /// Whether this block is a checkpoint block.
    pub is_checkpoint: bool,
    /// The order of this block in the checkpoint sequence, if it is a checkpoint block.
    pub checkpoint_order: Option<u16>,
}

/// Runs the two-step polycodes decode (`decode_track_code` then
/// `decode_track_data`) for any version implementing the trait system.
/// This part genuinely doesn't vary by version - it's the reason a trait
/// rewrite of polycodes is worth it here.
pub fn decode_track_bytes<T: PcTrack>(export_string: &str) -> anyhow::Result<T> {
    let (_name, _info, track_data) = T::decode_track_code(export_string)
        .ok_or_else(|| anyhow!("failed to decode track code"))?;
    decode_track_data::<T>(&track_data).ok_or_else(|| anyhow!("failed to decode track data"))
}

/// The one piece of metadata every supported version has, but which isn't
/// part of polycodes' own `Track` trait (each version's `Metadata` is an
/// opaque associated type there).
pub trait MinOffset {
    /// Returns the `(min_x, min_y, min_z)` offset of this track's blocks, which is version-specific and must be folded into absolute coordinates for raycasting.
    fn min_offset(&self) -> (i32, i32, i32);
}

impl MinOffset for V5TrackMetadata {
    fn min_offset(&self) -> (i32, i32, i32) {
        (self.min_x, self.min_y, self.min_z)
    }
}

impl MinOffset for V6TrackMetadata {
    fn min_offset(&self) -> (i32, i32, i32) {
        (self.min_x, self.min_y, self.min_z)
    }
}

/// Flattens any polycodes track whose metadata carries a min-offset and
/// whose blocks are `(u32, u32, u32)` positioned with a `Direction`-based
/// extra payload into placed, absolute-coordinate blocks.
///
/// This is the actual generic core: everything version-specific is
/// expressed as trait bounds (`MinOffset`, `Into<Direction>`), so adding a
/// third version means adding two small trait impls above, not touching
/// this function.
pub fn extract_blocks<T, D>(track: T) -> Vec<Block>
where
    T: PcTrack,
    T::Metadata: MinOffset,
    <T::Part as PcPart>::Block: PcBlock<Coord = u32, Extra = (D, u8, Option<u16>, Option<u32>)>,
    D: Into<Direction>,
{
    let (min_x, min_y, min_z) = track.meta().min_offset();
    let mut index = -1;

    track
        .parts()
        .into_iter()
        .flat_map(|part| {
            let part_id = part.id();
            part.blocks().into_iter().map(move |block| {
                let (x, y, z) = block.pos();
                let (dir, _color, cp_order, start_order) = block.extra_data();
                let dir_int = match dir.into() {
                    Direction::XNeg => 0,
                    Direction::XPos => 1,
                    Direction::YNeg => 2,
                    Direction::YPos => 3,
                    Direction::ZNeg => 4,
                    Direction::ZPos => 5,
                };
                index += 1;

                Block {
                    index: index as usize,
                    part_id,
                    x: x as i32 + min_x,
                    y: y as i32 + min_y,
                    z: z as i32 + min_z,
                    rotation: block.rot(),
                    dir: dir_int,
                    is_start: start_order.is_some(),
                    start_order,
                    is_checkpoint: cp_order.is_some(),
                    checkpoint_order: cp_order,
                }
            })
        })
        .collect()
}
