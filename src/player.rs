use rltk::{VirtualKeyCode, Rltk, Point, console};
use specs::prelude::*;
use crate::{RunState, CombatStats, WantsToMelee, gamelog, GameLog};

use super::{Position, Player, TileType, State, Viewshed, Map, Item, WantsToPickupItem};
use super::{xy_idx};
use std::cmp::{min, max};

pub fn try_move_player(delta_x : i32, delta_y : i32, ecs : &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();
    let mut viewsheds = ecs.write_storage::<Viewshed>();
    let combat_stats = ecs.read_storage::<CombatStats>();
    let map = ecs.fetch::<Map>();
    let entities = ecs.entities();
    let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();

    for (entity, _player, pos, viewshed) in (&entities, &mut players, &mut positions, &mut viewsheds).join() {
        if pos.x + delta_x < 1 || pos.x + delta_x > map.width-1 || pos.y + delta_y < 1 || pos.y + delta_y > map.height-1 { return; }
        let destination_idx = xy_idx(pos.x + delta_x, pos.y + delta_y);

        for potential_target in map.tile_content[destination_idx].iter() {
            let target = combat_stats.get(*potential_target);
            if let Some(_target) = target {
                wants_to_melee.insert(entity, WantsToMelee { 
                    target: *potential_target 
                })
                .expect("Add target failed");
            }
        }

        if !map.blocked[destination_idx] {
            pos.x = min(79, max(0, pos.x + delta_x));
            pos.y = min(49, max(0, pos.y + delta_y));

            let mut ppos = ecs.write_resource::<Point>();
            ppos.x = pos.x;
            ppos.y = pos.y;

            viewshed.dirty = true;
        }
    }
}

fn get_item(ecs: &mut World) {
    let player_pos = ecs.fetch::<Point>();
    let player_entity = ecs.fetch::<Entity>();
    let entities = ecs.entities();
    let items = ecs.read_storage::<Item>();
    let positions = ecs.read_storage::<Position>();
    let mut gamelog = ecs.fetch_mut::<GameLog>();

    let mut target_item : Option<Entity> = None;
    for (item_entity, _item, position) in (&entities, &items, &positions).join() {
        if position.x == player_pos.x && position.y == player_pos.y {
            target_item = Some(item_entity);
        }
    }

    match target_item {
        None => gamelog.entries.push("There is nothing here to pick up.".to_string()),
        Some(item) => {
            let mut pickup = ecs.write_storage::<WantsToPickupItem>();
            pickup.insert(*player_entity, WantsToPickupItem { collected_by: *player_entity, item}).expect("Unable to insert want to pickup");
        }
    }

}

pub fn player_input(gs : &mut State, ctx : &mut Rltk) -> RunState {
    match ctx.key {
        None => {
            return RunState::AwaitingInput
        },
        Some(key) => match key {
            VirtualKeyCode::Left |
            VirtualKeyCode::A => try_move_player(-1, 0, &mut gs.ecs),
            VirtualKeyCode::Right |
            VirtualKeyCode::D => try_move_player(1, 0, &mut gs.ecs),
            VirtualKeyCode::Up |
            VirtualKeyCode::W => try_move_player(0, -1, &mut gs.ecs),
            VirtualKeyCode::Down |
            VirtualKeyCode::S => try_move_player(0, 1, &mut gs.ecs),

            // Diagonals
            VirtualKeyCode::Numpad9 | 
            VirtualKeyCode::E => try_move_player(1, -1, &mut gs.ecs),
            VirtualKeyCode::Numpad7 |
            VirtualKeyCode::Q => try_move_player(-1, -1, &mut gs.ecs),
            VirtualKeyCode::Numpad3 |
            VirtualKeyCode::C => try_move_player(1, 1, &mut gs.ecs),
            VirtualKeyCode::Numpad1 |
            VirtualKeyCode::Z => try_move_player(-1, 1, &mut gs.ecs),
            
            // Picking up items
            VirtualKeyCode::G => get_item(&mut gs.ecs),
            VirtualKeyCode::I => return RunState::ShowInventory,
            VirtualKeyCode::F => return RunState::ShowDropItem,

            // Save and Quit
            VirtualKeyCode::Escape => return RunState::SaveGame,
            
            _ => {
                return RunState::AwaitingInput
            }
        }
    }
    RunState::PlayerTurn
}