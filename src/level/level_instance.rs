use std::collections::VecDeque;

use bevy::{math::Vec3A, prelude::*, render::primitives::Aabb, utils::HashMap};

use crate::{
    gameplay::{snake_pluggin::Snake, undo::LevelEntityUpdateEvent},
    utils::ray_intersects_aabb,
};
use bevy_prototype_debug_lines::DebugShapes;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum LevelEntityType {
    Food,
    Spike,
    Wall,
    Snake(i32),
}

#[derive(Resource)]
pub struct LevelInstance {
    occupied_cells: HashMap<IVec3, LevelEntityType>,
}

impl LevelInstance {
    pub fn new() -> Self {
        LevelInstance {
            occupied_cells: HashMap::new(),
        }
    }

    pub fn is_empty(&self, position: IVec3) -> bool {
        !self.occupied_cells.contains_key(&position)
    }

    pub fn is_empty_or_spike(&self, position: IVec3) -> bool {
        !self.occupied_cells.contains_key(&position) || self.is_spike(position)
    }

    pub fn set_empty(&mut self, position: IVec3) -> Option<LevelEntityType> {
        self.occupied_cells.remove(&position)
    }

    pub fn mark_position_occupied(&mut self, position: IVec3, value: LevelEntityType) {
        self.occupied_cells.insert(position, value);
    }

    pub fn is_food(&self, position: IVec3) -> bool {
        matches!(
            self.occupied_cells.get(&position),
            Some(LevelEntityType::Food)
        )
    }

    pub fn get(&self, position: IVec3) -> Option<&LevelEntityType> {
        self.occupied_cells.get(&position)
    }

    pub fn is_spike(&self, position: IVec3) -> bool {
        matches!(
            self.occupied_cells.get(&position),
            Some(LevelEntityType::Spike)
        )
    }

    pub fn is_snake(&self, position: IVec3) -> Option<i32> {
        let walkable = self.occupied_cells.get(&position);
        match walkable {
            Some(LevelEntityType::Snake(index)) => Some(*index),
            _ => None,
        }
    }

    /// Move a snake forward.
    /// Set the old tail location empty and mark the new head as occupied.
    /// Returns a list of updates to the walkable cells that can be undone.
    pub fn move_snake_forward(
        &mut self,
        snake: &Snake,
        direction: IVec3,
    ) -> Vec<LevelEntityUpdateEvent> {
        let mut updates: Vec<LevelEntityUpdateEvent> = Vec::with_capacity(2);
        let new_position = snake.head_position() + direction;

        let old_value = self.set_empty(snake.tail_position()).unwrap();
        self.mark_position_occupied(new_position, LevelEntityType::Snake(snake.index()));

        updates.push(LevelEntityUpdateEvent::ClearPosition(
            snake.tail_position(),
            old_value,
        ));
        updates.push(LevelEntityUpdateEvent::FillPosition(new_position));

        updates
    }

    /// Move a snake by an offset:
    /// Set the old locations are empty and mark the new locations as occupied.
    /// Returns a list of updates to the walkable cells that can be undone.
    pub fn move_snake(&mut self, snake: &Snake, offset: IVec3) -> Vec<LevelEntityUpdateEvent> {
        let mut updates: VecDeque<LevelEntityUpdateEvent> =
            VecDeque::with_capacity(2 * snake.len());

        for (position, _) in snake.parts() {
            let old_value = self.set_empty(*position).unwrap();
            updates.push_front(LevelEntityUpdateEvent::ClearPosition(*position, old_value));
        }
        for (position, _) in snake.parts() {
            let new_position = *position + offset;
            self.mark_position_occupied(new_position, LevelEntityType::Snake(snake.index()));
            updates.push_front(LevelEntityUpdateEvent::FillPosition(new_position));
        }

        updates.into()
    }

    pub fn eat_food(&mut self, position: IVec3) -> Vec<LevelEntityUpdateEvent> {
        let old_value = self.set_empty(position).unwrap();
        vec![LevelEntityUpdateEvent::ClearPosition(position, old_value)]
    }

    pub fn grow_snake(&mut self, snake: &Snake) -> Vec<LevelEntityUpdateEvent> {
        let (tail_position, tail_direction) = snake.tail();
        let new_part_position = tail_position - tail_direction;

        self.mark_position_occupied(new_part_position, LevelEntityType::Snake(snake.index()));
        vec![LevelEntityUpdateEvent::FillPosition(new_part_position)]
    }

    pub fn clear_snake_positions(&mut self, snake: &Snake) -> Vec<LevelEntityUpdateEvent> {
        let mut updates: Vec<LevelEntityUpdateEvent> = Vec::with_capacity(snake.len());
        for (position, _) in snake.parts() {
            let old_value = self.set_empty(*position).unwrap();
            updates.push(LevelEntityUpdateEvent::ClearPosition(*position, old_value));
        }
        updates
    }

    pub fn mark_snake_positions(&mut self, snake: &Snake) -> Vec<LevelEntityUpdateEvent> {
        let mut updates: Vec<LevelEntityUpdateEvent> = Vec::with_capacity(snake.len());
        for (position, _) in snake.parts() {
            self.mark_position_occupied(*position, LevelEntityType::Snake(snake.index()));
            updates.push(LevelEntityUpdateEvent::FillPosition(*position));
        }
        updates
    }

    pub fn undo_updates(&mut self, updates: &Vec<LevelEntityUpdateEvent>) {
        for update in updates {
            match update {
                LevelEntityUpdateEvent::ClearPosition(position, value) => {
                    self.mark_position_occupied(*position, *value);
                }
                LevelEntityUpdateEvent::FillPosition(position) => {
                    self.set_empty(*position);
                }
            }
        }
    }

    pub fn can_push_snake(&self, snake: &Snake, direction: IVec3) -> bool {
        snake.parts().iter().all(|(position, _)| {
            self.is_empty(*position + direction)
                || self.is_snake_with_index(*position + direction, snake.index())
        })
    }

    pub fn is_snake_with_index(&self, position: IVec3, snake_index: i32) -> bool {
        let walkable = self.occupied_cells.get(&position);
        match walkable {
            Some(LevelEntityType::Snake(index)) => *index == snake_index,
            _ => false,
        }
    }

    pub fn is_wall_or_spike(&self, position: IVec3) -> bool {
        matches!(
            self.occupied_cells.get(&position),
            Some(LevelEntityType::Wall)
        ) || matches!(
            self.occupied_cells.get(&position),
            Some(LevelEntityType::Spike)
        )
    }

    pub fn get_distance_to_ground(&self, position: IVec3, snake_index: i32) -> i32 {
        let mut distance = 1;

        const ARBITRARY_HIGH_DISTANCE: i32 = 50;

        let mut current_position = position + IVec3::NEG_Y;
        while self.is_empty_or_spike(current_position)
            || self.is_snake_with_index(current_position, snake_index)
        {
            current_position += IVec3::NEG_Y;
            distance += 1;

            // There is no ground below.
            if current_position.y <= 0 {
                return ARBITRARY_HIGH_DISTANCE;
            }
        }

        distance
    }

    pub fn find_first_free_cell_on_ray(&self, ray: Ray, shapes: &mut DebugShapes) -> Option<IVec3> {
        let aabb = self.compute_bounds();

        // we extend the bounds by one unit so that there will always be a empty cell before the first non empty cell.
        // In the case where the start ray is outside of the bounds of course, TODO: check if this is not the case.
        let bound_min: Vec3 = (aabb.min() - Vec3A::ONE).into();
        let bound_max: Vec3 = (aabb.max() + Vec3A::ONE).into();

        shapes
            .cuboid()
            .position(aabb.center.into())
            .size(bound_max - bound_min)
            .duration(5.0)
            .color(Color::GREEN);

        let Some([t_min, t_max]) = ray_intersects_aabb(ray, &Aabb::from_min_max(bound_min, bound_max), &Mat4::IDENTITY) else {
            return None;
        };

        let ray_start = ray.origin + ray.direction * t_min;
        let ray_end = ray.origin + ray.direction * t_max;

        shapes
            .cuboid()
            .position(ray_start)
            .size(0.2 * Vec3::ONE)
            .duration(5.0)
            .color(Color::RED);

        shapes
            .cuboid()
            .position(ray_end)
            .size(0.2 * Vec3::ONE)
            .duration(5.0)
            .color(Color::RED);

        let sign = |x| {
            if x > 0.0 {
                1
            } else if x < 0.0 {
                -1
            } else {
                0
            }
        };
        let frac_1 = |x: f32| 1.0 - x + x.floor();

        let mut t_max_x;
        let mut t_max_y;
        let mut t_max_z;

        let x1 = ray_start.x + 0.5;
        let y1 = ray_start.y + 0.5;
        let z1 = ray_start.z + 0.5;
        let x2 = ray_end.x + 0.5;
        let y2 = ray_end.y + 0.5;
        let z2 = ray_end.z + 0.5;

        let dx = sign(x2 - x1);
        let t_delta_x = if dx != 0 {
            10000000.0f32.min((dx as f32) / (x2 - x1))
        } else {
            10000000.0
        };
        if dx > 0 {
            t_max_x = t_delta_x * frac_1(x1);
        } else {
            t_max_x = t_delta_x * x1.fract()
        }

        let dy = sign(y2 - y1);
        let t_delta_y = if dy != 0 {
            10000000.0f32.min((dy as f32) / (y2 - y1))
        } else {
            10000000.0
        };
        if dy > 0 {
            t_max_y = t_delta_y * frac_1(y1);
        } else {
            t_max_y = t_delta_y * y1.fract()
        }

        let dz = sign(z2 - z1);
        let t_delta_z = if dz != 0 {
            10000000.0f32.min((dz as f32) / (z2 - z1))
        } else {
            10000000.0
        };
        if dz > 0 {
            t_max_z = t_delta_z * frac_1(z1);
        } else {
            t_max_z = t_delta_z * z1.fract()
        }

        let start_position = Vec3::new(x1 - 0.5, y1 - 0.5, z1 - 0.5).round().as_ivec3();
        let mut current_position = start_position;
        let mut prev_position = current_position;
        println!("========");
        println!("Start: {:?}", start_position);

        loop {
            shapes
                .cuboid()
                .position(current_position.as_vec3())
                .size(Vec3::ONE)
                .duration(5.0);

            if t_max_x < t_max_y {
                if t_max_x < t_max_z {
                    current_position.x += dx;
                    t_max_x += t_delta_x;
                } else {
                    current_position.z += dz;
                    t_max_z += t_delta_z;
                }
            } else if t_max_y < t_max_z {
                current_position.y += dy;
                t_max_y += t_delta_y;
            } else {
                current_position.z += dz;
                t_max_z += t_delta_z;
            }
            if t_max_x > 1.0 && t_max_y > 1.0 && t_max_z > 1.0 {
                break;
            }

            // process voxel here

            if !self.is_empty(current_position) {
                println!(
                    "Non Empty: {:?}, prev:{:?}",
                    current_position, prev_position
                );
                return Some(prev_position);
            }

            prev_position = current_position;
        }

        None
    }

    pub fn compute_bounds(&self) -> Aabb {
        let mut min = 1000 * IVec3::ONE;
        let mut max = 1000 * IVec3::NEG_ONE;

        self.occupied_cells.iter().for_each(|(position, _)| {
            min = min.min(*position);
            max = max.max(*position);
        });

        Aabb::from_min_max(
            min.as_vec3() - 0.5 * Vec3::ONE,
            max.as_vec3() + 0.5 * Vec3::ONE,
        )
    }
}
