use specs::prelude::*;

use crate::{Viewshed, Monster, Map, Position, xy_idx, RunState, WantsToMelee, Confusion};
use rltk::Point;


/// Responsible for pursuing a player
pub struct MonsterAI {}

impl<'a> System<'a> for MonsterAI {
    #[allow(clippy::type_complexity)]
    type SystemData = ( WriteExpect<'a, Map>,
                        ReadExpect<'a, Point>,
                        ReadExpect<'a, Entity>,
                        ReadExpect<'a, RunState>,
                        Entities<'a>,
                        WriteStorage<'a, Viewshed>,
                        ReadStorage<'a, Monster>,
                        WriteStorage<'a, Position>,
                        WriteStorage<'a, WantsToMelee>,
                        WriteStorage<'a, Confusion>,
                    );
    
    fn run(&mut self, data : Self::SystemData) {
        let (mut map, player_pos, 
            player_entity, runstate, 
            entities, mut viewshed, 
            monster, mut position, 
            mut wants_to_melee, mut confused) = data;

        if *runstate != RunState::MonsterTurn {
            return;
        }

        for (entity, mut viewshed, _monster, mut pos) in (&entities, &mut viewshed, &monster, &mut position).join() {
            let mut can_act = true;

            // skip turns if get confused
            let is_confused = confused.get_mut(entity);
            if let Some(i_am_confused) = is_confused {
                i_am_confused.turns -= 1;
                if i_am_confused.turns < 1 {
                    confused.remove(entity);
                }
                can_act = false;
            }
            
            // count distance to the player and move to him
            if can_act {
                let distance = rltk::DistanceAlg::Pythagoras.distance2d(Point::new(pos.x, pos.y), *player_pos);

                if distance < 1.5 {
                    wants_to_melee.insert(entity, WantsToMelee { target: *player_entity }).expect("Unable to insert attack");
                } else if viewshed.visible_tiles.contains(&*player_pos) {
                    // Path to the player
                    let path = rltk::a_star_search(
                        xy_idx(pos.x, pos.y),
                        xy_idx(player_pos.x, player_pos.y),
                        &mut *map
                    );

                    if path.success && path.steps.len() > 1 {
                        let mut idx = xy_idx(pos.x, pos.y);
                        map.blocked[idx] = false;
                        pos.x = path.steps[1] as i32 % map.width;
                        pos.y = path.steps[1] as i32 / map.width;
                        idx = xy_idx(pos.x, pos.y);
                        map.blocked[idx] = true;
                        viewshed.dirty = true;
                    }
                }
            }
        }
    }
}
