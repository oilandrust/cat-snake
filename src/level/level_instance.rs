use std::collections::VecDeque;

use bevy::{prelude::*, render::primitives::Aabb, utils::HashMap};

use crate::{
    gameplay::{snake_pluggin::Snake, undo::LevelEntityUpdateEvent},
    utils::ray_intersects_aabb,
};

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

    pub fn occupied_cells(&self) -> &HashMap<IVec3, LevelEntityType> {
        &self.occupied_cells
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

    pub fn find_first_free_cell_on_ray(&self, ray: Ray) -> Option<IVec3> {
        let aabb = self.compute_bounds();

        let Some(intersection) = ray_intersects_aabb(ray, &aabb, &Mat4::IDENTITY) else {
            return None;
        };

        let intersection_0 = (ray.origin + intersection[0] * ray.direction)
            .round()
            .as_ivec3();
        let intersection_1 = (ray.origin + intersection[1] * ray.direction)
            .round()
            .as_ivec3();

        let cells_on_ray = bresenham_3d(
            intersection_0.x,
            intersection_0.y,
            intersection_0.z,
            intersection_1.x,
            intersection_1.y,
            intersection_1.z,
        );

        let first_non_enpty_cell = cells_on_ray
            .iter()
            .position(|position| !self.is_empty(*position));

        first_non_enpty_cell.map(|index| cells_on_ray[index - 1])
    }

    pub fn compute_bounds(&self) -> Aabb {
        let mut min = 1000 * IVec3::ONE;
        let mut max = 1000 * IVec3::NEG_ONE;

        self.occupied_cells.iter().for_each(|(position, _)| {
            min = min.min(*position);
            max = max.max(*position);
        });

        Aabb::from_min_max(min.as_vec3() - Vec3::ONE, max.as_vec3() + Vec3::ONE)
    }
}

fn bresenham_3d(x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) -> Vec<IVec3> {
    let mut points = vec![];

    let mut point = IVec3::new(x1, y1, z1);

    let dx = x2 - x1;
    let dy = y2 - y1;
    let dz = z2 - z1;

    let x_inc = if dx < 0 { -1 } else { 1 };
    let l = dx.abs();

    let y_inc = if dy < 0 { -1 } else { 1 };
    let m = dy.abs();

    let z_inc = if dz < 0 { -1 } else { 1 };
    let n = dz.abs();

    let dx2 = l << 1;
    let dy2 = m << 1;
    let dz2 = n << 1;

    if (l >= m) && (l >= n) {
        let mut err_1 = dy2 - l;
        let mut err_2 = dz2 - l;
        for _ in 0..l {
            points.push(point);
            if err_1 > 0 {
                point.y += y_inc;
                err_1 -= dx2;
            }
            if err_2 > 0 {
                point.z += z_inc;
                err_2 -= dx2;
            }
            err_1 += dy2;
            err_2 += dz2;
            point.x += x_inc;
        }
    } else if (m >= l) && (m >= n) {
        let mut err_1 = dx2 - m;
        let mut err_2 = dz2 - m;
        for _ in 0..m {
            points.push(point);
            if err_1 > 0 {
                point.x += x_inc;
                err_1 -= dy2;
            }
            if err_2 > 0 {
                point.z += z_inc;
                err_2 -= dy2;
            }
            err_1 += dx2;
            err_2 += dz2;
            point.y += y_inc;
        }
    } else {
        let mut err_1 = dy2 - n;
        let mut err_2 = dx2 - n;
        for _ in 0..n {
            points.push(point);
            if err_1 > 0 {
                point.y += y_inc;
                err_1 -= dz2;
            }
            if err_2 > 0 {
                point.x += x_inc;
                err_2 -= dz2;
            }
            err_1 += dy2;
            err_2 += dx2;
            point.z += z_inc;
        }
    }

    points.push(IVec3::new(x2, y2, z2));
    points
}
