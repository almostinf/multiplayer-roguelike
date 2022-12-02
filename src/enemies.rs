use specs::prelude::*;
use crate::{idx_xy};

use super::{Position, Name, Enemy};

#[derive(Default)]
pub struct EnemySystem {
    pub enemies_pos : Vec<(String, i32)>,
}

impl<'a> System<'a> for EnemySystem {
    type SystemData = ( ReadStorage<'a, Name>,
                        ReadStorage<'a, Enemy>,
                        WriteStorage<'a, Position>,
                    );

    fn run(&mut self, data : Self::SystemData) {
        let (names, enemy, mut pos) = data;

        for (name, _e, p) in (&names, &enemy, &mut pos).join() {
            for pare in self.enemies_pos.iter() {
                if pare.0 == name.name {
                    p.x = idx_xy(pare.1).0;
                    p.y = idx_xy(pare.1).1;
                }
            }
        }
    }
}
