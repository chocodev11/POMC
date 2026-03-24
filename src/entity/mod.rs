use std::collections::HashMap;

pub struct ItemEntity {
    pub position: glam::DVec3,
    pub item_name: String,
    pub count: i32,
    pub age: u32,
    pub bob_offset: f32,
}

struct PickupAnimation {
    item_name: String,
    start_pos: glam::DVec3,
    target_pos: glam::DVec3,
    bob_offset: f32,
    age: u32,
    life: u32,
}

pub struct PickupRenderInfo {
    pub item_name: String,
    pub position: glam::DVec3,
    pub bob_offset: f32,
    pub age: u32,
}

const PICKUP_LIFE: u32 = 3;

pub struct EntityStore {
    items: HashMap<i32, ItemEntity>,
    pickups: Vec<PickupAnimation>,
}

impl EntityStore {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            pickups: Vec::new(),
        }
    }

    pub fn spawn_item(&mut self, id: i32, position: glam::DVec3) {
        let bob_offset =
            ((id as u32).wrapping_mul(2654435761)) as f32 / u32::MAX as f32 * std::f32::consts::TAU;
        self.items.insert(
            id,
            ItemEntity {
                position,
                item_name: String::new(),
                count: 1,
                age: 0,
                bob_offset,
            },
        );
    }

    pub fn set_item_data(&mut self, id: i32, item_name: String, count: i32) {
        if let Some(entity) = self.items.get_mut(&id) {
            entity.item_name = item_name;
            entity.count = count;
        }
    }

    pub fn move_delta(&mut self, id: i32, dx: f64, dy: f64, dz: f64) {
        if let Some(entity) = self.items.get_mut(&id) {
            entity.position.x += dx;
            entity.position.y += dy;
            entity.position.z += dz;
        }
    }

    pub fn teleport(&mut self, id: i32, position: glam::DVec3) {
        if let Some(entity) = self.items.get_mut(&id) {
            entity.position = position;
        }
    }

    pub fn pickup(&mut self, item_id: i32, target_pos: glam::DVec3) {
        if let Some(entity) = self.items.remove(&item_id) {
            if !entity.item_name.is_empty() {
                self.pickups.push(PickupAnimation {
                    item_name: entity.item_name,
                    start_pos: entity.position,
                    target_pos,
                    bob_offset: entity.bob_offset,
                    age: entity.age,
                    life: 0,
                });
            }
        }
    }

    pub fn remove(&mut self, ids: &[i32]) {
        for &id in ids {
            self.items.remove(&id);
        }
    }

    pub fn tick(&mut self) {
        for entity in self.items.values_mut() {
            entity.age += 1;
        }
        for pickup in &mut self.pickups {
            pickup.life += 1;
        }
        self.pickups.retain(|p| p.life < PICKUP_LIFE);
    }

    pub fn visible_items(&self, camera_pos: glam::DVec3, max_dist: f64) -> Vec<&ItemEntity> {
        let max_dist_sq = max_dist * max_dist;
        self.items
            .values()
            .filter(|e| {
                !e.item_name.is_empty() && e.position.distance_squared(camera_pos) < max_dist_sq
            })
            .collect()
    }

    pub fn active_pickups(&self, partial_tick: f32) -> Vec<PickupRenderInfo> {
        self.pickups
            .iter()
            .map(|p| {
                let t = (p.life as f32 + partial_tick) / PICKUP_LIFE as f32;
                let pos = p.start_pos.lerp(p.target_pos, t as f64);
                PickupRenderInfo {
                    item_name: p.item_name.clone(),
                    position: pos,
                    bob_offset: p.bob_offset,
                    age: p.age,
                }
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.pickups.clear();
    }
}
