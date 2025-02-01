// shoutout to chatgpt
use crate::instance::Instance;

pub struct Humanoid {
    address: usize,
    mem: Option<Instance>,
}

pub struct HumanoidOffsets {
    health_display_distance: f64,
    name_display_distance: f64,
    health: f64,
    max_health: f64,
    walk_speed: [f64; 2],
}

pub static OFFSETS_HUMANOID: HumanoidOffsets = HumanoidOffsets {
    health_display_distance: 400.0,
    name_display_distance: 436.0,
    health: 396.0,
    max_health: 428.0,
    walk_speed: [456.0, 928.0],
};

impl Humanoid {
    pub fn new(rbx: &Instance) -> Self {
        Humanoid {
            address: rbx.address,
            mem: Some(rbx.clone()),
        }
    }

    pub fn get_health(&self) -> f32 {
        if self.address < 1000 {
            return 0.0;
        }
        let mem = self.mem.as_ref().unwrap();
        let hp = mem.read_float(self.address + OFFSETS_HUMANOID.health as usize);
        hp
    }

    pub fn get_max_health(&self) -> f32 {
        if self.address < 1000 {
            return 0.0;
        }
        let mem = self.mem.as_ref().unwrap();
        let hp = mem.read_float(self.address + OFFSETS_HUMANOID.max_health as usize);
        hp
    }

    pub fn set_health(&self, health: f32) {
        if self.address < 1000 {
            return;
        }
        let mem = self.mem.as_ref().unwrap();
        mem.write_float(self.address + OFFSETS_HUMANOID.health as usize, health);
    }

    pub fn set_max_health(&self, health: f32) {
        if self.address < 1000 {
            return;
        }
        let mem = self.mem.as_ref().unwrap();
        mem.write_float(self.address + OFFSETS_HUMANOID.max_health as usize, health);
    }

    pub fn walk_speed(&self, speed: f32) {
        if self.address < 1000 {
            return;
        }
        let mem = self.mem.as_ref().unwrap();
        mem.write_float(self.address + OFFSETS_HUMANOID.walk_speed[0] as usize, speed);
        mem.write_float(self.address + OFFSETS_HUMANOID.walk_speed[1] as usize, speed);
    }
}
