use std::collections::HashMap;

pub struct ItemEntity {
    pub position: glam::DVec3,
    pub item_name: String,
    pub count: i32,
    pub age: u32,
    pub bob_offset: f32,
}

pub struct EntityStore {
    items: HashMap<i32, ItemEntity>,
}

impl EntityStore {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
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

    pub fn remove(&mut self, ids: &[i32]) {
        for &id in ids {
            self.items.remove(&id);
        }
    }

    pub fn tick(&mut self) {
        for entity in self.items.values_mut() {
            entity.age += 1;
        }
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

    pub fn clear(&mut self) {
        self.items.clear();
    }
}
