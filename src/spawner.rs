use std::collections::HashMap;

use rltk::{RGB, RandomNumberGenerator};
use specs::prelude::*;
use crate::{AreaOfEffect, EquipmentSlot, Equippable, MeleePowerBonus, DefenseBonus};
use super::random_table::*;

use super::{CombatStats, Player, Renderable, Name, Position, Viewshed, Monster, BlocksTile, Rect, Item, ProvidesHealing, Consumable, Ranged, InflictDamage, Confusion, SerializeMe};
use super::MAPWIDTH;
use specs::saveload::{MarkedBuilder, SimpleMarker};

const MAX_MONSTERS : i32 = 4;
// const MAX_ITEMS: i32 = 2;

/// Spawns the player and returns his entity object
pub fn player(ecs: &mut World, player_x: i32, player_y: i32) -> Entity {
    ecs
        .create_entity()
        .with(Position{x: player_x, y: player_y})
        .with(Renderable {
            glyph: rltk::to_cp437('@'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 0,
        })
        .with(Player {})
        .with(Viewshed{visible_tiles: Vec::new(), range: 8, dirty: true})
        .with(Name {name: "Player".to_string()})
        .with(CombatStats{max_hp: 30, hp: 30, defense: 2, power: 5})
        .marked::<SimpleMarker<SerializeMe>>()
        .build()
}

/// Spawns a random monster at a given location
pub fn random_monster(ecs: &mut World, x: i32, y: i32) {
    let roll: i32;
    {
        let mut rng =  ecs.write_resource::<RandomNumberGenerator>();
        roll = rng.roll_dice(1,2);
    }
    match roll {
        1 => { orc(ecs, x, y)}
        _ => { goblin(ecs, x, y)}
    }
}

fn orc(ecs: &mut World, x: i32, y: i32) {
    monster(ecs, x, y, rltk::to_cp437('o'), "Orc");
}

fn goblin(ecs: &mut World, x: i32, y: i32) {
    monster(ecs, x, y, rltk::to_cp437('g'), "Goblin");
}

fn monster<S : ToString>(ecs: &mut World, x: i32, y: i32, glyph: rltk::FontCharType, name: S) {
    ecs.create_entity()
        .with(Position {x, y})
        .with(Renderable {
            glyph,
            fg: RGB::named(rltk::RED),
            bg: RGB::named(rltk::BLACK),
            render_order: 1,
        })
        .with(Viewshed {visible_tiles: Vec::new(), range: 8, dirty: true})
        .with(Monster{})
        .with(Name{name: name.to_string()})
        .with(BlocksTile{})
        .with(CombatStats{max_hp: 16, hp: 16, defense: 1, power: 4})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

#[allow(clippy::map_entry)]
pub fn spawn_room(ecs: &mut World, room: &Rect, map_depth : i32) {
    let spawn_table = room_table(map_depth);
    let mut spawn_points : HashMap<usize, String> = HashMap::new();

    // Scope to keep the borrow happy
    {
        let mut rng = ecs.write_resource::<RandomNumberGenerator>();
        let num_spawns = rng.roll_dice(1, MAX_MONSTERS + 3) + (map_depth - 1) - 3;

        for _i in 0..num_spawns {
            let mut added = false;
            let mut tries = 0;
            while !added && tries < 20 {
                let x = (room.x1 + rng.roll_dice(1, i32::abs(room.x2 - room.x1))) as usize;
                let y = (room.y1 + rng.roll_dice(1, i32::abs(room.y2 - room.y1))) as usize;
                let idx = (y * MAPWIDTH) + x;
                if !spawn_points.contains_key(&idx) {
                    spawn_points.insert(idx, spawn_table.roll(&mut rng));
                    added = true;
                } else {
                    tries += 1;
                }
            }
        }
    }
    // Actually spawn the monsters
    for spawn in spawn_points.iter() {
        let x = (*spawn.0 % MAPWIDTH) as i32;
        let y = (*spawn.0 / MAPWIDTH) as i32;

        match spawn.1.as_ref() {
            "Goblin" => goblin(ecs, x, y),
            "Orc" => orc(ecs, x, y),
            "Health Potion" => health_potion(ecs, x, y),
            "Fireball Scroll" => fireball_scroll(ecs, x, y),
            "Confusion Scroll" => confusion_scroll(ecs, x, y),
            "Magic Missible Scroll" => magic_missible_scroll(ecs, x, y),
            "Dagger" => dagger(ecs, x, y),
            "Shield" => shield(ecs, x, y),
            "Longsword" => longsword(ecs, x, y),
            "Tower Shield" => tower_shield(ecs, x, y),
            _ => {}
        }   
    }
}

fn health_potion(ecs: &mut World, x : i32, y: i32) {
    ecs.create_entity()
        .with(Position{x, y})
        .with(Renderable {
            glyph: rltk::to_cp437('¡'),
            fg: RGB::named(rltk::MAGENTA),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {name: "Health Potion".to_string()})
        .with(Item {})
        .with(ProvidesHealing {heal_amount: 8})
        .with(Consumable {})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn magic_missible_scroll(ecs : &mut World, x : i32, y : i32) {
    ecs.create_entity()
        .with(Position {x, y})
        .with(Renderable {
            glyph : rltk::to_cp437(')'),
            fg : RGB::named(rltk::CYAN),
            bg : RGB::named(rltk::BLACK),
            render_order : 2,
        })
        .with(Name {name : "Magic Missible Scroll".to_string()})
        .with(Item {})
        .with(Consumable {})
        .with(Ranged {range : 6})
        .with(InflictDamage {damage : 8})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn random_item(ecs : &mut World, x : i32, y : i32) {
    let roll : i32;
    {
        let mut rng = ecs.write_resource::<RandomNumberGenerator>();
        roll = rng.roll_dice(1, 4);
    }
    match roll {
        1 => health_potion(ecs, x, y),
        2 => fireball_scroll(ecs, x, y),
        3 => confusion_scroll(ecs, x, y),
        _ => magic_missible_scroll(ecs, x, y)
    }
}

fn fireball_scroll(ecs : &mut World, x : i32, y : i32) {
    ecs.create_entity()
        .with(Position {x, y})
        .with(Renderable {
            glyph : rltk::to_cp437(')'),
            fg : RGB::named(rltk::ORANGE),
            bg : RGB::named(rltk::BLACK),
            render_order : 2,
        })
        .with(Name {name : "Fireball Scroll".to_string()})
        .with(Item {})
        .with(Consumable {})
        .with(Ranged {range : 6})
        .with(InflictDamage {damage : 20})
        .with(AreaOfEffect {radius : 3})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn confusion_scroll(ecs : &mut World, x : i32, y : i32) {
    ecs.create_entity()
        .with(Position {x, y})
        .with(Renderable {
            glyph : rltk::to_cp437(')'),
            fg : RGB::named(rltk::PINK),
            bg : RGB::named(rltk::BLACK),
            render_order : 2,
        })
        .with(Name {name : "Confusion Scroll".to_string()})
        .with(Item {})
        .with(Consumable {})
        .with(Ranged {range : 6})
        .with(Confusion {turns : 4})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn dagger(ecs : &mut World, x : i32, y : i32) {
    ecs.create_entity()
        .with(Position {x, y})
        .with(Renderable {
            glyph : rltk::to_cp437('/'),
            fg : RGB::named(rltk::CYAN),
            bg : RGB::named(rltk::BLACK),
            render_order : 2,
        })
        .with(Name {name : "Dagger".to_string()})
        .with(Item {})
        .with(Equippable {slot : EquipmentSlot::Melee})
        .with(MeleePowerBonus {power : 2})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn shield(ecs : &mut World, x : i32, y : i32) {
    ecs.create_entity()
        .with(Position {x, y})
        .with(Renderable {
            glyph : rltk::to_cp437('('),
            fg : RGB::named(rltk::CYAN),
            bg : RGB::named(rltk::BLACK),
            render_order : 2,
        })
        .with(Name {name : "Shield".to_string()})
        .with(Item {})
        .with(Equippable {slot : EquipmentSlot::Melee})
        .with(DefenseBonus {defense : 1})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn longsword(ecs : &mut World, x : i32, y : i32) {
    ecs.create_entity()
        .with(Position {x, y})
        .with(Renderable {
            glyph : rltk::to_cp437('/'),
            fg : RGB::named(rltk::YELLOW),
            bg : RGB::named(rltk::BLACK),
            render_order : 2,
        })
        .with(Name {name : "Longsword".to_string()})
        .with(Item {})
        .with(Equippable {slot : EquipmentSlot::Melee})
        .with(MeleePowerBonus { power : 4})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn tower_shield(ecs : &mut World, x : i32, y : i32) {
    ecs.create_entity()
        .with(Position {x, y})
        .with(Renderable {
            glyph : rltk::to_cp437('('),
            fg : RGB::named(rltk::YELLOW),
            bg : RGB::named(rltk::BLACK),
            render_order : 2,
        })
        .with(Name { name : "Tower Shield".to_string()})
        .with(Item {})
        .with(Equippable { slot : EquipmentSlot::Shield})
        .with(DefenseBonus {defense : 3})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}
