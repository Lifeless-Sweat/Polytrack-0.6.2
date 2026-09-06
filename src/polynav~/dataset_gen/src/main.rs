use std::{
    collections::HashMap,
    env::args,
    fs::{File, create_dir_all, exists, read_to_string},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use parry3d::{glamx::Quat, math::Vec3};
use polysim::{
    physics::{PolyTrackPhysics, create_engine},
    simulation::{PlayerController, PreparedTrack, SimulationWorker},
};
use polytrack_codes::{v5::V5Track, v6::V6Track};
use polyworld::{World, assets};
use rayon::prelude::*;
use serde_json::{from_str, to_writer};
use wasmtime::Module;

use crate::{
    dataset::{Entry, LeaderboardEntry, Run},
    replay::Replay,
    track::{decode_track_bytes, extract_blocks},
};

mod dataset;
mod replay;
mod track;

fn main() {
    let track_id = args()
        .nth(1)
        .expect("Please provide a track ID as the first argument.");

    if !exists("../data/out").expect("Failed to read output directory.") {
        create_dir_all("../data/out").expect("Failed to create output directory.");
    }

    let map2code_str =
        read_to_string("../data/map2code.json").expect("Failed to read map2code.json.");
    let map2code: HashMap<String, String> =
        from_str(&map2code_str).expect("Failed to parse map2code.json.");
    let code = map2code
        .get(&track_id)
        .expect("Track ID not found in map2code.json.");

    let (blocks, world) = if code.starts_with("PolyTrack24") {
        let track = decode_track_bytes::<V6Track>(code).expect("Failed to decode track bytes.");
        let world = World::from_track_code_with_assets::<V6Track>(code, assets())
            .expect("Failed to create world from track code.");
        (extract_blocks(track), world)
    } else if code.starts_with("PolyTrack14") {
        let track = decode_track_bytes::<V5Track>(code).expect("Failed to decode track bytes.");
        let world = World::from_track_code_with_assets::<V5Track>(code, assets())
            .expect("Failed to create world from track code.");
        (extract_blocks(track), world)
    } else {
        panic!("Unsupported track version.");
    };

    let output_path = format!("../data/out/{}.json", track_id);

    let leaderboard_str = read_to_string("../data/top2k.json").expect("Failed to read top2k.json.");
    let leaderboard: Vec<LeaderboardEntry> =
        from_str(&leaderboard_str).expect("Failed to parse top2k.json.");

    let engine = create_engine();
    let module =
        Module::from_file(&engine, "../physics.wasm").expect("Failed to load physics module.");

    let filtered: Vec<&LeaderboardEntry> =
        leaderboard.iter().filter(|e| e.track == track_id).collect();
    let len = filtered.len();
    println!("Processing {} runs for track {}...", len, track_id);

    let results: Vec<Run> = filtered
        .par_iter()
        .map_init(
            || {
                let physics = PolyTrackPhysics::from_module(&engine, &module)
                    .expect("Failed to create physics engine.");
                let prepared =
                    PreparedTrack::from_export_string(code).expect("Failed to prepare track.");
                let mut worker = SimulationWorker::new(physics, prepared)
                    .expect("Failed to create simulation worker.");
                worker
                    .init()
                    .expect("Failed to initialize simulation worker.");
                worker.create_car(0).expect("Failed to create car.");
                worker
            },
            |worker, lb_entry| {
                println!(
                    "Processing run {} of {} for track {}...",
                    lb_entry.position, len, track_id
                );
                let replay = Replay::deserialize(
                    &URL_SAFE_NO_PAD
                        .decode(&lb_entry.recording)
                        .expect("Failed to decode recording"),
                )
                .expect("Failed to deserialize replay.");

                let mut run = Run {
                    visited_order: vec![],
                    time: lb_entry.frames,
                };

                worker.reset_car(0).expect("Failed to reset car.");
                let mut last_index = None;

                for frame in 0..lb_entry.frames {
                    let input_frame = replay.get_frame(frame as u32);
                    worker
                        .set_car_controls(
                            0,
                            PlayerController {
                                up: input_frame.up,
                                right: input_frame.right,
                                down: input_frame.down,
                                left: input_frame.left,
                                reset: input_frame.reset,
                            },
                        )
                        .expect("Failed to set car controls.");

                    let state = worker.update_car(0).expect("Failed to update car.");
                    let car_quat = state.quaternion;
                    let down_vec =
                        Quat::from_xyzw(car_quat[0], car_quat[1], car_quat[2], car_quat[3])
                            .mul_vec3(Vec3::new(0.0, -1.0, 0.0));

                    if let Some((index, _, _)) =
                        world.raycast(Vec3::from_array(state.position), down_vec, 1.0)
                        && last_index != Some(index)
                    {
                        run.visited_order.push(index as usize);
                        last_index = Some(index);
                    }
                }

                run
            },
        )
        .collect();

    let mut seen_runs: HashMap<Vec<usize>, u64> = HashMap::new();
    let mut entry = Entry {
        blocks,
        runs: vec![],
    };

    for run in results {
        let best_time = seen_runs
            .get(&run.visited_order)
            .copied()
            .unwrap_or(u64::MAX);
        if run.time < best_time {
            seen_runs.insert(run.visited_order.clone(), run.time);
            entry.runs.push(run);
        }
    }

    let output_file = File::create(&output_path).expect("Failed to create output file.");
    to_writer(&output_file, &entry).expect("Failed to write output file.");
}
