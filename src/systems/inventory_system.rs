use specs::prelude::*;

use crate::{WantsToPickupItem, Name, InBackpack, Position, gamelog::GameLog};
use crate::{WantsToUseItem, ProvidesHealing, CombatStats, WantsToDropItem, Consumable};
use crate::{SufferDamage, InflictDamage, xy_idx, Map, AreaOfEffect, Confusion, Equippable, Equipped, WantsToRemoveItem};


/// Responsible for picking up objects
pub struct ItemCollectSystem {}


impl<'a> System<'a> for ItemCollectSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = ( ReadExpect<'a, Entity>,
                        WriteExpect<'a, GameLog>,
                        WriteStorage<'a, WantsToPickupItem>,
                        WriteStorage<'a, Position>,
                        ReadStorage<'a, Name>,
                        WriteStorage<'a, InBackpack>
                    );
    
    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, mut wants_pickup, mut position, names, mut backpack) = data;

        for pickup in wants_pickup.join() {
            position.remove(pickup.item);
            backpack.insert(pickup.item, InBackpack { owner: pickup.collected_by}).expect("Unable to insert backpack entry");

            if pickup.collected_by == *player_entity {
                gamelog.entries.push(format!("You pick up the {}.", names.get(pickup.item).unwrap().name));
            }
        }
        wants_pickup.clear();
    }
}

/// Responsible for the use of objects and the execution of their effects
pub struct ItemUseSystem {}


impl<'a> System<'a> for ItemUseSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = ( ReadExpect<'a, Entity>,
                        WriteExpect<'a, GameLog>,
                        ReadExpect<'a, Map>,
                        Entities<'a>,
                        WriteStorage<'a, WantsToUseItem>,
                        ReadStorage<'a, Name>,
                        ReadStorage<'a, Consumable>,
                        ReadStorage<'a, ProvidesHealing>,
                        ReadStorage<'a, InflictDamage>,
                        WriteStorage<'a, CombatStats>,
                        WriteStorage<'a, SufferDamage>,
                        ReadStorage<'a, AreaOfEffect>,
                        WriteStorage<'a, Confusion>,
                        ReadStorage<'a, Equippable>,
                        WriteStorage<'a, Equipped>,
                        WriteStorage<'a, InBackpack>,
                    );
    
    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, 
                map , entities, 
                mut wants_use, names, 
                consumables, healing, inflict_damage, 
                mut combat_stats, mut suffer_damage, aoe, 
                mut confused, equippable, mut equipped, mut backpack) = data;
        
        for (entity, useitem) in (&entities, &wants_use).join() {
            let mut used_item = true;

            // targeting
            let mut targets : Vec<Entity> = Vec::new();
            match useitem.target {
                None => targets.push(*player_entity),
                Some (target) => {
                    let area_effect = aoe.get(useitem.item);
                    match area_effect {
                        None => {
                            // single target in tile
                            let idx = xy_idx(target.x, target.y);
                            for mob in map.tile_content[idx].iter() {
                                targets.push(*mob);
                            }
                        }
                        Some(area_effect) => {
                            // aoe
                            let mut blast_tiles = rltk::field_of_view(target, area_effect.radius, &*map);
                            blast_tiles.retain(|p| p.x > 0 && p.x < map.width - 1 && p.y > 0 && p.y < map.height - 1);
                            for tile_idx in blast_tiles.iter() {
                                let idx = xy_idx(tile_idx.x, tile_idx.y);
                                for mob in map.tile_content[idx].iter() {
                                    targets.push(*mob);
                                }
                            }
                        }
                    }
                }
            }

            // if it is equippable, then we want to equip it - and unequip whatever else it that slot
            let item_equippable = equippable.get(useitem.item);
            match item_equippable {
                None => {},
                Some(can_equip) => {
                    let target_slot = can_equip.slot;
                    let target = targets[0];

                    // remove any items the target has in the item's slot 
                    let mut to_unequip : Vec<Entity> = Vec::new();
                    for (item_entity, already_equipped, name) in (&entities, &equipped, &names).join() {
                        if already_equipped.owner == target && already_equipped.slot == target_slot {
                            to_unequip.push(item_entity);
                            if target == *player_entity {
                                gamelog.entries.push(format!("You unequip {}", name.name));
                            }
                        }
                    }

                    for item in to_unequip.iter() {
                        equipped.remove(*item);
                        backpack.insert(*item, InBackpack { owner: target }).expect("Unable to insert backpack entry");
                    }

                    // wield the item
                    equipped.insert(useitem.item, Equipped { owner: target, slot: target_slot }).expect("Unable to insert equipped component");
                    backpack.remove(useitem.item);
                    if target == *player_entity {
                        gamelog.entries.push(format!("You equip {}", names.get(useitem.item).unwrap().name));
                    }
                }
            }

            // healing
            let item_healing = healing.get(useitem.item);
            match item_healing {
                None => {}
                Some(healer) => {
                    for target in targets.iter() {
                        let stats = combat_stats.get_mut(*target);
                        if let Some(stats) = stats {
                            stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
                            if entity == *player_entity {
                                gamelog.entries.push(format!("You drink the {}, healing {} hp.", names.get(useitem.item).unwrap().name, healer.heal_amount));
                            }
                        }
                    }
                }
            }

            // damage
            let item_damage = inflict_damage.get(useitem.item);
            match item_damage {
                None => {},
                Some(damage) => {
                    used_item = false;
                    for mob in targets.iter() {
                        SufferDamage::new_damage(&mut suffer_damage, *mob, damage.damage);
                        if entity == *player_entity {
                            let mob_name = names.get(*mob).unwrap();
                            let item_name = names.get(useitem.item).unwrap();
                            gamelog.entries.push(format!("You use {} on {}, inflicting {} hp.", item_name.name, mob_name.name, damage.damage));
                        }
                        used_item = true;
                    }
                }
            }

            let mut add_confusion = Vec::new();
            {
                let causes_confusion = confused.get(useitem.item);
                match causes_confusion {
                    None => {}
                    Some(confusion) => {
                        for mob in targets.iter() {
                            add_confusion.push((*mob, confusion.turns));
                            if entity == *player_entity {
                                let mob_name = names.get(*mob).unwrap();
                                let item_name = names.get(useitem.item).unwrap();
                                gamelog.entries.push(format!("You can use {} on {}, confusion them.", item_name.name, mob_name.name));
                            }
                        }
                        used_item = true;
                    }
                }
            }

            for mob in add_confusion.iter() {
                confused.insert(mob.0, Confusion { turns: mob.1 }).expect("Unable to insert status");
            }

            // If its a consumable, we delete it on use
            if used_item {
                let consumable = consumables.get(useitem.item);
                match consumable {
                    None => {}
                    Some(_) => {
                        entities.delete(useitem.item).expect("Delete failed");
                    }
                }
            }
        }
        wants_use.clear();
    }
}


/// Responsible for the dropping items
pub struct ItemDropSystem {}


impl<'a> System<'a> for ItemDropSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = ( ReadExpect<'a, Entity>,
                        WriteExpect<'a, GameLog>,
                        Entities<'a>,
                        WriteStorage<'a, WantsToDropItem>,
                        ReadStorage<'a, Name>,
                        WriteStorage<'a, Position>,
                        WriteStorage<'a, InBackpack>
                    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, entities, mut wants_drop, names, mut positions, mut backpack) = data;

        for (entity, to_drop) in (&entities, &wants_drop).join() {
            let mut dropper_pos : Position = Position { x: 0, y: 0 };
            {
                let dropped_pos = positions.get(entity).unwrap();
                dropper_pos.x = dropped_pos.x;
                dropper_pos.y = dropped_pos.y;
            }

            positions.insert(to_drop.item, Position { x: dropper_pos.x, y: dropper_pos.y }).expect("Unable to insert position");
            backpack.remove(to_drop.item);

            if entity == *player_entity {
                gamelog.entries.push(format!("You drop the {}.", names.get(to_drop.item).unwrap().name));
            }
        }
        wants_drop.clear();
    }
}


/// Responsible for removing items to the backpack
pub struct ItemRemoveSystem {}


impl<'a> System<'a> for ItemRemoveSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
            Entities<'a>,
            WriteStorage<'a, WantsToRemoveItem>,
            WriteStorage<'a, Equipped>,
            WriteStorage<'a, InBackpack>,
    );

    fn run(&mut self, data : Self::SystemData) {
        let (entities, mut wants_remove, mut equipped, mut backpack) = data;
        
        for (entity, to_remove) in (&entities, &wants_remove).join() {
            equipped.remove(to_remove.item);
            backpack.insert(to_remove.item, InBackpack { owner: entity }).expect("Unable to insert backpack");
        }

        wants_remove.clear();
    }
}
