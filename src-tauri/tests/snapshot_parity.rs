use deskbuddy_lib::pet::{Pet, Phase, StdPetRng};
use rand::SeedableRng;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Frame {
    t: f64,
    pos_x: f64,
    pos_y: f64,
    state: String,
    mirror: bool,
    phase: String,
}

fn fixture() -> Vec<Frame> {
    let json = include_str!("fixtures/pet_trajectory_seed42.json");
    serde_json::from_str(json).expect("parse fixture")
}

#[test]
fn trajectory_matches_committed_snapshot() {
    let expected = fixture();
    let mut pet = Pet::new(1920.0, 1080.0, 134.0, 146.0, true, 0.0);
    let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));

    for (i, exp) in expected.iter().enumerate() {
        pet.tick(exp.t, &mut rng);
        assert!(
            (pet.pos_x - exp.pos_x).abs() < 1e-9,
            "pos_x mismatch at frame {i}: {} vs {}",
            pet.pos_x,
            exp.pos_x
        );
        assert!(
            (pet.pos_y - exp.pos_y).abs() < 1e-9,
            "pos_y mismatch at frame {i}: {} vs {}",
            pet.pos_y,
            exp.pos_y
        );
        assert_eq!(pet.state.as_str(), exp.state, "state mismatch at frame {i}");
        assert_eq!(pet.mirror, exp.mirror, "mirror mismatch at frame {i}");
        let phase_str = match pet.phase {
            Phase::Pause => "Pause",
            Phase::Walk => "Walk",
        };
        assert_eq!(phase_str, exp.phase, "phase mismatch at frame {i}");
    }
    assert_eq!(
        expected.len(),
        600,
        "fixture should have 600 frames (10s @ 60fps)"
    );
}
