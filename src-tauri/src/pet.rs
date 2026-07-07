use std::f64;

pub const STRIDE_PER_LOOP: f64 = 0.8;
pub const REST_CHANCE: f64 = 0.62;
pub const PAUSE_MIN: f64 = 1.5;
pub const PAUSE_MAX: f64 = 13.0;
pub const PAUSE_MEAN: f64 = 4.2;
pub const STROLL_MIN_FRAC: f64 = 0.45;
pub const STROLL_MIN_PX: f64 = 110.0;
pub const STROLL_BIAS: f64 = 0.85;
pub const ARRIVE_EPS: f64 = 2.0;
pub const FPS: u32 = 60;
pub const GRAVITY_PX_S2: f64 = 1800.0;
pub const FEET_DROP_PX: f64 = 4.0;
pub const TASKBAR_H: f64 = 48.0;
pub const LOOP_MS_F: f64 = 1100.0;

pub const IDLE_STATES: [&str; 7] = [
    "idle", "idle", "idle", "idle", "waving", "waiting", "review",
];

pub fn anim_speed(state: &str) -> f64 {
    match state {
        "idle" => 1.6,
        "waving" => 1.3,
        "waiting" => 1.4,
        "review" => 1.3,
        "run_right" => 1.0,
        "run_left" => 1.0,
        "jump" => 0.8,
        _ => 1.0,
    }
}

pub trait PetRng {
    fn next_f64(&mut self) -> f64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Pause,
    Walk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetState {
    Idle,
    RunRight,
    RunLeft,
    Wave,
    Jump,
    Failed,
    Waiting,
    Review,
    Running,
}

impl PetState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PetState::Idle => "idle",
            PetState::RunRight => "run_right",
            PetState::RunLeft => "run_left",
            PetState::Wave => "wave",
            PetState::Jump => "jump",
            PetState::Failed => "failed",
            PetState::Waiting => "waiting",
            PetState::Review => "review",
            PetState::Running => "running",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PetSnapshot {
    pub pos_x: f64,
    pub pos_y: f64,
    pub state: PetState,
    pub mirror: bool,
    pub phase: Phase,
}

pub struct Pet {
    pub sw: f64,
    pub sh: f64,
    pub fw: f64,
    pub fh: f64,
    pub pos_x: f64,
    pub pos_y: f64,
    pub vel_y: f64,
    pub ground_y: f64,
    pub phase: Phase,
    pub pause_until: f64,
    pub walk_target: f64,
    pub walk_speed: f64,
    pub roam_dir: i32,
    pub state: PetState,
    pub mirror: bool,
    pub anim_idx: usize,
    pub run_enabled: bool,
    pub dragging: bool,
    pub last_tick: f64,
}

impl Pet {
    pub fn new(sw: f64, sh: f64, fw: f64, fh: f64, run_enabled: bool, now: f64) -> Self {
        let ground_y = sh - TASKBAR_H - fh + FEET_DROP_PX;
        Pet {
            sw,
            sh,
            fw,
            fh,
            pos_x: 80.0,
            pos_y: ground_y,
            vel_y: 0.0,
            ground_y,
            phase: Phase::Pause,
            pause_until: now + 2.0,
            walk_target: 80.0,
            walk_speed: 0.0,
            roam_dir: 0,
            state: PetState::Idle,
            mirror: false,
            anim_idx: 0,
            run_enabled,
            dragging: false,
            last_tick: now,
        }
    }

    pub fn recompute_ground(&mut self) {
        self.ground_y = self.sh - TASKBAR_H - self.fh + FEET_DROP_PX;
    }

    pub fn tick<R: PetRng>(&mut self, now: f64, rng: &mut R) {
        let dt = (now - self.last_tick).clamp(0.0, 0.05);
        self.last_tick = now;
        if self.run_enabled && !self.dragging {
            self.roam(now, dt, rng);
        }
        self.recompute_ground();
    }

    fn roam<R: PetRng>(&mut self, now: f64, dt: f64, rng: &mut R) {
        if self.pos_y < self.ground_y - 1.0 {
            self.vel_y += GRAVITY_PX_S2 * dt;
            self.pos_y += self.vel_y * dt;
            if self.pos_y >= self.ground_y {
                self.pos_y = self.ground_y;
                self.vel_y = 0.0;
                self.land(now, rng);
            }
            self.state = PetState::Jump;
            return;
        }
        match self.phase {
            Phase::Pause if now >= self.pause_until => self.plan(now, rng),
            Phase::Walk => {
                let rem = self.walk_target - self.pos_x;
                let step = self.walk_speed * dt;
                if rem.abs() <= ARRIVE_EPS.max(step) {
                    self.pos_x = self.walk_target;
                    self.begin_pause(now, rng);
                } else {
                    let dir = if rem > 0.0 { 1.0 } else { -1.0 };
                    self.pos_x += dir * step;
                    self.roam_dir = if rem > 0.0 { 1 } else { -1 };
                }
            }
            _ => {}
        }
        self.pos_x = self.pos_x.max(10.0).min(self.sw - self.fw - 10.0);
        if self.phase == Phase::Walk {
            self.state = PetState::RunRight;
            self.mirror = self.roam_dir < 0;
        } else {
            self.state = PetState::Idle;
            self.mirror = (self.pos_x + self.fw / 2.0) < (self.sw / 2.0);
        }
    }

    fn land<R: PetRng>(&mut self, now: f64, rng: &mut R) {
        self.phase = Phase::Pause;
        self.roam_dir = 0;
        self.state = pick_idle(rng);
        self.anim_idx = 0;
        self.pause_until = now + 0.8;
    }

    fn pick_target<R: PetRng>(&mut self, rng: &mut R) -> f64 {
        let m = 10.0;
        let lb = m;
        let rb = (self.sw - self.fw - m).max(m);
        let sp = rb - lb;
        if sp <= 4.0 {
            return self.pos_x;
        }
        let rl = self.pos_x - lb;
        let rr = rb - self.pos_x;
        let r1 = rng.next_f64();
        let go_r = (r1 < STROLL_BIAS) == (rr >= rl);
        let room = if go_r { rr } else { rl }.max(0.0);
        let md = room.min((sp * STROLL_MIN_FRAC).max(STROLL_MIN_PX));
        let r2 = rng.next_f64();
        let extra = (room - md).max(0.0) * r2;
        let sign = if go_r { 1.0 } else { -1.0 };
        self.pos_x + (md + extra) * sign
    }

    fn begin_pause<R: PetRng>(&mut self, now: f64, rng: &mut R) {
        self.phase = Phase::Pause;
        self.roam_dir = 0;
        self.state = pick_idle(rng);
        self.anim_idx = 0;
        self.pause_until = now + exp_dwell(rng);
    }

    fn begin_walk<R: PetRng>(&mut self, now: f64, rng: &mut R) {
        let t = self.pick_target(rng);
        let diff = t - self.pos_x;
        if diff.abs() < ARRIVE_EPS {
            self.begin_pause(now, rng);
            return;
        }
        self.walk_target = t;
        self.phase = Phase::Walk;
        self.walk_speed = self.fw * STRIDE_PER_LOOP / (LOOP_MS_F / 1000.0);
        self.roam_dir = if diff > 0.0 { 1 } else { -1 };
    }

    fn plan<R: PetRng>(&mut self, now: f64, rng: &mut R) {
        let r = rng.next_f64();
        if r < REST_CHANCE {
            self.begin_pause(now, rng);
        } else {
            self.begin_walk(now, rng);
        }
    }

    pub fn on_press(&mut self, _x_root: f64, _y_root: f64) {
        self.dragging = true;
    }

    pub fn on_drag(&mut self, x_root: f64, y_root: f64, drag_ox: f64, drag_oy: f64) {
        if !self.dragging {
            return;
        }
        self.pos_x = (x_root - drag_ox).max(0.0).min(self.sw - self.fw);
        self.pos_y = (y_root - drag_oy).max(0.0).min(self.sh - self.fh);
    }

    pub fn on_release<R: PetRng>(&mut self, now: f64, rng: &mut R) {
        self.dragging = false;
        if self.pos_y < self.ground_y - 1.0 {
            self.vel_y = 0.0;
        } else {
            self.pos_y = self.ground_y;
            self.begin_pause(now, rng);
            self.pause_until = now + 0.8;
        }
    }

    pub fn snapshot(&self) -> PetSnapshot {
        PetSnapshot {
            pos_x: self.pos_x,
            pos_y: self.pos_y,
            state: self.state,
            mirror: self.mirror,
            phase: self.phase,
        }
    }
}

fn pick_idle<R: PetRng>(rng: &mut R) -> PetState {
    let idx = (rng.next_f64() * (IDLE_STATES.len() as f64)) as usize;
    match IDLE_STATES[idx.min(IDLE_STATES.len() - 1)] {
        "idle" => PetState::Idle,
        "waving" => PetState::Wave,
        "waiting" => PetState::Waiting,
        "review" => PetState::Review,
        _ => PetState::Idle,
    }
}

fn exp_dwell<R: PetRng>(rng: &mut R) -> f64 {
    let u = 1.0 - rng.next_f64();
    let v = -u.ln() * PAUSE_MEAN;
    v.clamp(PAUSE_MIN, PAUSE_MAX)
}

pub struct StdPetRng(pub rand::rngs::StdRng);

impl PetRng for StdPetRng {
    fn next_f64(&mut self) -> f64 {
        use rand::Rng;
        self.0.gen::<f64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn make_pet(now: f64) -> Pet {
        Pet::new(1920.0, 1080.0, 134.0, 146.0, true, now)
    }

    #[test]
    fn initial_state() {
        let p = make_pet(0.0);
        assert_eq!(p.state, PetState::Idle);
        assert_eq!(p.phase, Phase::Pause);
        assert!((p.pos_x - 80.0).abs() < 1e-9);
        assert!((p.pause_until - 2.0).abs() < 1e-9);
        assert!((p.ground_y - (1080.0 - 48.0 - 146.0 + 4.0)).abs() < 1e-9);
    }

    #[test]
    fn stays_idle_during_initial_pause() {
        let mut p = make_pet(0.0);
        let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));
        for i in 1..60 {
            p.tick(i as f64 * 0.016, &mut rng);
        }
        assert_eq!(
            p.state,
            PetState::Idle,
            "should stay idle until pause_until"
        );
        assert_eq!(p.phase, Phase::Pause);
    }

    #[test]
    fn bounds_respected_over_long_run() {
        let mut p = make_pet(0.0);
        let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));
        for i in 1..(60 * 30) {
            let now = (i as f64) * 0.016;
            p.tick(now, &mut rng);
            assert!(p.pos_x >= 0.0, "pos_x below 0 at tick {i}");
            assert!(p.pos_x <= p.sw, "pos_x beyond sw at tick {i}");
            assert!(p.pos_y >= 0.0, "pos_y below 0 at tick {i}");
            assert!(
                p.pos_y <= p.ground_y + 1.0,
                "pos_y above ground at tick {i}"
            );
        }
    }

    #[test]
    fn walk_eventually_happens() {
        let mut p = make_pet(0.0);
        let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(7));
        let mut walked = false;
        for i in 1..(60 * 20) {
            let now = (i as f64) * 0.016;
            p.tick(now, &mut rng);
            if p.phase == Phase::Walk {
                walked = true;
                break;
            }
        }
        assert!(walked, "pet should eventually walk");
    }

    #[test]
    fn jump_when_above_ground() {
        let mut p = make_pet(0.0);
        let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));
        p.pos_y = p.ground_y - 100.0;
        p.vel_y = 0.0;
        p.tick(0.016, &mut rng);
        assert_eq!(p.state, PetState::Jump);
        assert!(p.vel_y > 0.0, "gravity should accelerate downward");
    }

    #[test]
    fn drag_overrides_position() {
        let mut p = make_pet(0.0);
        p.on_press(100.0, 200.0);
        assert!(p.dragging);
        p.on_drag(500.0, 700.0, 20.0, 30.0);
        assert!((p.pos_x - 480.0).abs() < 1e-9);
        assert!((p.pos_y - 670.0).abs() < 1e-9);
    }

    #[test]
    fn release_on_ground_starts_pause() {
        let mut p = make_pet(0.0);
        let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));
        p.on_press(100.0, p.ground_y);
        p.on_drag(100.0, p.ground_y, 0.0, 0.0);
        p.on_release(5.0, &mut rng);
        assert_eq!(p.phase, Phase::Pause);
        assert!(!p.dragging);
    }

    #[test]
    fn release_in_air_zeroes_velocity() {
        let mut p = make_pet(0.0);
        let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));
        p.on_press(100.0, p.ground_y);
        p.pos_y = p.ground_y - 50.0;
        p.on_release(5.0, &mut rng);
        assert!((p.vel_y).abs() < 1e-9);
        assert!(!p.dragging);
    }

    #[test]
    fn determinism_same_seed_same_trajectory() {
        fn run(seed: u64) -> Vec<PetSnapshot> {
            let mut p = make_pet(0.0);
            let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(seed));
            let mut out = Vec::new();
            for i in 1..(60 * 10) {
                let now = (i as f64) * 0.016;
                p.tick(now, &mut rng);
                out.push(p.snapshot());
            }
            out
        }
        let a = run(42);
        let b = run(42);
        assert_eq!(a.len(), b.len());
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert_eq!(x.state, y.state, "state mismatch at {i}");
            assert!((x.pos_x - y.pos_x).abs() < 1e-9, "pos_x at {i}");
            assert!((x.pos_y - y.pos_y).abs() < 1e-9, "pos_y at {i}");
        }
    }

    #[test]
    fn exp_dwell_clamped() {
        let mut rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(1));
        for _ in 0..1000 {
            let d = exp_dwell(&mut rng);
            assert!(
                (PAUSE_MIN..=PAUSE_MAX).contains(&d),
                "dwell {d} out of bounds"
            );
        }
    }

    #[test]
    fn anim_speed_known_states() {
        assert!((anim_speed("idle") - 1.6).abs() < 1e-9);
        assert!((anim_speed("jump") - 0.8).abs() < 1e-9);
        assert!((anim_speed("run_right") - 1.0).abs() < 1e-9);
        assert!((anim_speed("unknown") - 1.0).abs() < 1e-9);
    }
}
