use deskbuddy_lib::pet::{Pet, Phase, StdPetRng};
use rand::SeedableRng;
use serde::Serialize;

#[derive(Serialize)]
struct Frame {
    t: f64,
    pos_x: f64,
    pos_y: f64,
    state: &'static str,
    mirror: bool,
    phase: &'static str,
}

fn main() {
    let mut pet = Pet::new(1920.0, 1080.0, 134.0, 146.0, true, 0.0);
    let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));
    let mut frames: Vec<Frame> = Vec::with_capacity(60 * 10);
    for i in 1..=(60 * 10) {
        let t = (i as f64) * 0.016;
        pet.tick(t, &mut rng);
        frames.push(Frame {
            t,
            pos_x: pet.pos_x,
            pos_y: pet.pos_y,
            state: pet.state.as_str(),
            mirror: pet.mirror,
            phase: match pet.phase {
                Phase::Pause => "Pause",
                Phase::Walk => "Walk",
            },
        });
    }
    let json = serde_json::to_string_pretty(&frames).expect("serialize");
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pet_trajectory_seed42.json");
    std::fs::create_dir_all(out.parent().unwrap()).expect("dir");
    std::fs::write(&out, json).expect("write");
    println!("wrote {} frames to {}", frames.len(), out.display());
}
