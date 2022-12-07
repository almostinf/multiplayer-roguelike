use rltk::{Rltk, GameState, Point, RGB};
use specs::prelude::*;

use url::Url;

mod map;
pub use map::*;
mod rect;
pub use rect::Rect;
mod player;
use player::*;
mod components;
pub use components::*;

mod gui;
pub use gui::*;
mod gamelog;
pub use gamelog::*;
mod spawner;
pub use spawner::*;
extern crate serde;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};
mod random_table;
pub use random_table::*;
mod client;
pub use client::*;
mod constants;
pub use constants::*;

pub mod systems;
pub use systems::damage_system::*;
pub use systems::enemy_system::*;
pub use systems::inventory_system::*;
pub use systems::map_indexing_system::*;
pub use systems::saveload_system::*;
pub use systems::visibility_system::*;
pub use systems::melee_combat_system::*;
pub use systems::monster_ai_system::*;


#[derive(PartialEq, Copy, Clone)]
/// The states of a finite automaton in which the player can be
pub enum RunState {
    EnteringName,
    AwaitingInput,
    ShowRating, 
    PreRun, 
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
    ShowTargeting {
        range : i32,
        item : Entity,
    },
    MainMenu {
        menu_selection : gui::MainMenuSelection,
    },
    NextLevel,
    ShowRemoveItem,
    GameOver,
}


/// Handler for ecs and clients
pub struct State {
    pub ecs : World,
    pub game_client : ClientHandler,
    pub player_name : String,
    pub enemies : Vec<String>,
}


impl GameState for State {

    /// Рandling the states of a end state machine every tick
    fn tick(&mut self, ctx : &mut Rltk) {
        ctx.cls();

        self.game_client.get_messages();

        self.delete_enemies();
        self.update_health_enemies();

        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        // draw map if current state is not in main menu
        match newrunstate {
            RunState::MainMenu {..} => {}
            _ => {
                draw_map(&self.ecs, ctx);

                let positions = self.ecs.read_storage::<Position>();
                let renderables = self.ecs.read_storage::<Renderable>();
                let map = self.ecs.fetch::<Map>();

                let mut data = (&positions, &renderables).join().collect::<Vec<_>>();
                data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
                
                // render player and monsters
                for (pos, render) in data.iter() {
                    let idx = xy_idx(pos.x, pos.y);
                    if map.visible_tiles[idx] {
                        ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
                    }
                }
                gui::draw_ui(&self.ecs, ctx);
            }
        }

        // handling end state machine
        match newrunstate {
            RunState::EnteringName => {
                self.run_systems();
                self.ecs.maintain();
                match gui::entering_name(ctx, &mut self.player_name) {
                    Ok(_) => {
                        // check if this name is used or not
                        let message = format!("{{\"__IS_NAME__\":\"{}\"}}", self.player_name).as_bytes().to_vec();
                        self.game_client.send_message(message);

                        let mut clone = self.game_client.messages.clone();
                        let mut response = clone.into_iter().filter(|(key, _)| *key == "__IS_NAME__").collect::<Vec<_>>();

                        while response.is_empty() {
                            self.game_client.get_messages();
                            clone = self.game_client.messages.clone();
                            response = clone.into_iter().filter(|(key, _)| *key == "__IS_NAME__").collect::<Vec<_>>();
                        }

                        if !response.is_empty() {
                            if response[0].1 == "T" {
                                println!("in true result\n");
                                let message = format!("{{\"__TRACK_ME__\":\"{} {}\"}}", self.player_name, 1).as_bytes().to_vec();
                                self.game_client.send_message(message);

                                let player = self.ecs.fetch::<Entity>();
                                let mut names = self.ecs.write_storage::<Name>();

                                let name = names.get_mut(*player);
                                
                                if let Some(_name) = name {
                                    _name.name = self.player_name.clone();
                                }

                                newrunstate = RunState::PreRun;
                            } else if response[0].1 == "F" {
                                ctx.print_color_centered(15, RGB::named(rltk::RED), RGB::named(rltk::BLACK), "This name is used. Please enter another");
                                self.player_name.clear();
                            }
                        }
                    }
                    Err(_) => ()
                }
            }
            RunState::PreRun => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = player_input(self, ctx);
            }
            RunState::ShowRating => {
                let result = gui::show_rating(self, ctx);
                match result {
                    ItemMenuResult::Cancel => newrunstate = RunState::MainMenu { menu_selection: MainMenuSelection::Rating },
                    _ => newrunstate = RunState::ShowRating,
                }
            }
            RunState::PlayerTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::MonsterTurn;
            }
            RunState::MonsterTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::ShowInventory => {
                let result = gui::show_inventory(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let is_ranged = self.ecs.read_storage::<Ranged>();
                        let is_item_ranged = is_ranged.get(item_entity);
                        if let Some(is_item_ranged) = is_item_ranged {
                            newrunstate = RunState::ShowTargeting { range: is_item_ranged.range, item: item_entity };
                        } else {
                            let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                            intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem { item: item_entity , target : None}).expect("Unable to insert intent");
                            newrunstate = RunState::PlayerTurn;
                        }
                    }
                }
            }
            RunState::ShowDropItem => {
                let result = gui::drop_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToDropItem { item: item_entity }).expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            } 
            RunState::ShowTargeting { range, item } => {
                let result = gui::ranged_target(self, ctx, range);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem { item, target : result.1 }).expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::MainMenu { .. } => {
                let result = gui::main_menu(self, ctx);
                match result {
                    gui::MainMenuResult::NoSelection { selected } => newrunstate = RunState::MainMenu { menu_selection: selected },
                    gui::MainMenuResult::Selected { selected } => {
                        match selected {
                            gui::MainMenuSelection::Play => newrunstate = RunState::PreRun,
                            gui::MainMenuSelection::SaveGame => {
                                systems::saveload_system::save_game(&mut self.ecs);
                                newrunstate = RunState::MainMenu{ menu_selection : gui::MainMenuSelection::Quit };
                            }
                            gui::MainMenuSelection::LoadGame => {
                                if systems::saveload_system::does_save_exist() {
                                    systems::saveload_system::load_game(&mut self.ecs);
                                    newrunstate = RunState::AwaitingInput;
                                    systems::saveload_system::delete_save();
                                } else {
                                    ctx.print_color_centered(30, RGB::named(rltk::RED), RGB::named(rltk::BLACK), "You don't have saves!!!");
                                }
                            }
                            gui::MainMenuSelection::Rating => newrunstate = RunState::ShowRating,
                            gui::MainMenuSelection::Quit => ::std::process::exit(0),
                        }
                    }
                }
            }
            RunState::NextLevel => {
                self.goto_next_level();
                newrunstate = RunState::PreRun;
            }
            RunState::ShowRemoveItem => {
                let result = gui::remove_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToRemoveItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToRemoveItem {item : item_entity}).expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::GameOver => {
                let result = gui::game_over(ctx);
                match result {
                    gui::GameOverResult::NoSelection => {}
                    gui::GameOverResult::QuitToMenu => {
                        self.game_over_cleanup();
                        ::std::process::exit(0);
                    }
                }
            }
        }

        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }

        systems::damage_system::delete_the_dead(&mut self.ecs);

        if self.game_client.messages.len() > 1 {
            self.game_client.messages.clear();
        }
    }
}


impl State {
    /// void all game systems
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem{};
        vis.run_now(&self.ecs);

        let enemies_pos = self.get_enemies_pos();
        let mut en = EnemySystem{enemies_pos};
        en.run_now(&self.ecs);

        let mut mob = MonsterAI{};
        mob.run_now(&self.ecs);
        let mut mapindex = MapIndexingSystem {};
        mapindex.run_now(&self.ecs);
        let mut melee = MeleeCombatSystem{};
        melee.run_now(&self.ecs);

        let mut damage = DamageSystem{ enemies : self.enemies.clone(), game_client : &mut self.game_client };
        damage.run_now(&self.ecs);

        let mut pickup = ItemCollectSystem{};
        pickup.run_now(&self.ecs);
        let mut potions = ItemUseSystem{};
        potions.run_now(&self.ecs);
        let mut drop_items = ItemDropSystem{};
        drop_items.run_now(&self.ecs);
        let mut item_remove = ItemRemoveSystem {};
        item_remove.run_now(&self.ecs);

        self.ecs.maintain();

    }

    /// Handles enemy movements, spawns it if it was not in the enemy vector
    fn get_enemies_pos(&mut self) -> Vec<(String, i32)> {
        // get current depth
        let current_depth;
        {
            let worldmap = self.ecs.read_resource::<Map>();
            current_depth = worldmap.depth;
        }

        let mut enemies_pos = Vec::<(String, i32)>::new();

        let clone = self.game_client.messages.clone();
        let response = clone.into_iter().filter(|(key, _)| *key == "__MESSAGE__").collect::<Vec<_>>();
        
        // parsing response
        for (_, value) in response {
            let split = value.split(' ');
            let v = split.collect::<Vec<&str>>();
            let name = v[0].to_string();
            let name_check = name.clone();
            let idx = v[1].parse::<i32>().unwrap();
            let level = v[2].parse::<i32>().unwrap();
            // println!("Name: {}, idx: {}, level: {}", name, idx, level);

            if current_depth == level {
                for n in self.enemies.iter() {
                    if name == *n && name != self.player_name {
                        enemies_pos.push((name, idx));
                        break;
                    }
                }

                if !self.enemies.contains(&name_check) && name_check != self.player_name {
                    self.enemies.push(name_check.clone());
                    let x = idx_xy(idx).0;
                    let y = idx_xy(idx).1;
                    spawner::enemy(&mut self.ecs, x, y, name_check);
                }
            }
        }
 
        enemies_pos
    }

    /// Removes all enemies when they change level
    fn delete_enemies(&mut self) {
        let clone = self.game_client.messages.clone();
        let response = clone.into_iter().filter(|(key, _)| *key == "__CHANGE__").collect::<Vec<_>>();

        let mut to_remove = Vec::<(String, i32)>::new();

        for (_, value) in response {
            let split = value.split(' ');
            let v = split.collect::<Vec<&str>>();
            if v.len() > 1 {
                // println!("to delete: {} {}", v[0].to_string(), value);
                let level = v[1].parse::<i32>().unwrap();
                to_remove.push((v[0].to_string(), level));
            }
        }

        let mut to_delete = Vec::<Entity>::new();

        let current_depth;
        {
            let worldmap = self.ecs.read_resource::<Map>();
            current_depth = worldmap.depth;
        }

        if !to_remove.is_empty() {
            let enemies = self.ecs.read_storage::<Enemy>();
            let names = self.ecs.read_storage::<Name>();
            
            let ents = enemies.fetched_entities();

            for e in ents.join() {
                let name = names.get(e).expect("Can't get name");
                if to_remove.iter().find(|(_name, _level)| *_name == *name.name && *_level == current_depth) != None {
                    to_delete.push(e);
                    let index = self.enemies.iter().position(|x| *x == *name.name).expect("Can't find position");
                    self.enemies.remove(index);
                }
            }
        }

        for e in to_delete {
            self.ecs.delete_entity(e).expect("Can't delete enemy on level change");
        }
    }

    /// Update enemies and player health
    fn update_health_enemies(&mut self) {
        let clone = self.game_client.messages.clone();
        let response = clone.into_iter().filter(|(key, _)| *key == "__DAMAGE__").collect::<Vec<_>>();

        let mut to_update = Vec::<(String, i32)>::new();

        for (_, value) in response {
            let split = value.split(' ');
            let v = split.collect::<Vec<&str>>();
            if v.len() > 1 {
                let health = v[1].parse::<i32>().unwrap();
                println!("to update: {} {}", v[0].to_string(), health);
                to_update.push((v[0].to_string(), health));
            }
        }
        
        if !to_update.is_empty() {
            let entities = self.ecs.entities();
            let names = self.ecs.read_storage::<Name>();
            let mut combat_stats = self.ecs.write_storage::<CombatStats>();

            for e in entities.join() {
                let name = names.get(e).expect("Can't get name of entity");

                if let Some(enemy) = to_update.iter().find(|(_name, _)| *_name == name.name) {
                    let old_health = combat_stats.get_mut(e).expect("Can't get combat stats");
                    old_health.hp = enemy.1;
                }
            }
        }
    }

    // Return all entities that should be remove on level change
    fn entities_to_remove_on_level_change(&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let player_entity = self.ecs.fetch::<Entity>();
        let equipped = self.ecs.read_storage::<Equipped>();

        let mut to_delete : Vec<Entity> = Vec::new();
        for entity in entities.join() {
            let mut should_delete = true;

            // don't delete the player
            let p = player.get(entity);
            if let Some(_p) = p {
                should_delete = false;
            }

            // don't delete the player's equipment
            let bp = backpack.get(entity);
            if let Some(bp) = bp {
                if bp.owner == *player_entity {
                    should_delete = false;
                }
            }

            let eq = equipped.get(entity);
            if let Some(eq) = eq {
                if eq.owner == *player_entity {
                    should_delete = false;
                }
            }

            if should_delete {
                to_delete.push(entity);
            }
        }

        // clear all enemies on previous level
        self.enemies.clear();

        to_delete
    }

    /// Moves to another level and handles responses from the server
    fn goto_next_level(&mut self) {

        // delete entities that are not the player or his/her equipment
        let to_delete = self.entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs.delete_entity(target).expect("Unable to delete entity");
        }
        
        // get current depth
        let current_depth;
        {
            let worldmap = self.ecs.read_resource::<Map>();
            current_depth = worldmap.depth;
        }

        let message = format!("{{\"__IS_MAP__\":\"{}\"}}", current_depth + 1).as_bytes().to_vec();
        self.game_client.send_message(message);

        self.game_client.get_messages();
        
        let mut clone = self.game_client.messages.clone();
        let mut response = clone.into_iter().filter(|(key, _)| *key == "__IS_MAP__").collect::<Vec<_>>();

        while response.is_empty() {
            self.game_client.get_messages();
            clone = self.game_client.messages.clone();
            response = clone.into_iter().filter(|(key, _)| *key == "__IS_MAP__").collect::<Vec<_>>();
        }

        if !response.is_empty() {
            if response[0].1 == "F" {
                {
                    // build a new map and place the player
                    let worldmap;
                    let current_depth;  
                    {
                        let mut worldmap_resource = self.ecs.write_resource::<Map>();
                        current_depth = worldmap_resource.depth;
                        *worldmap_resource = Map::new(current_depth + 1);
                        worldmap = worldmap_resource.clone();
                    }

                    // spawn rooms
                    for room in worldmap.rooms.iter().skip(1) {
                        spawner::spawn_room(&mut self.ecs, room, current_depth + 1);
                    }

                    // place the player and update resources
                    let (player_x, player_y) = worldmap.rooms[0].center();
                    let mut player_position = self.ecs.write_resource::<Point>();
                    *player_position = Point::new(player_x, player_y);
                    let mut position_components = self.ecs.write_storage::<Position>();
                    let player_entity = self.ecs.fetch::<Entity>();
                    let player_pos_comp = position_components.get_mut(*player_entity);
                    if let Some(player_pos_comp) = player_pos_comp {
                        player_pos_comp.x = player_x;
                        player_pos_comp.y = player_y;
                    }

                    // mark the player's visibility as dirty
                    let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
                    let vs = viewshed_components.get_mut(*player_entity);
                    if let Some(vs) = vs {
                        vs.dirty = true;
                    }

                    // notify the player and give them some health
                    let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
                    gamelog.entries.push("You descend to the next level, and take a moment to heal.".to_string());
                    let mut player_health_store = self.ecs.write_storage::<CombatStats>();
                    let player_health = player_health_store.get_mut(*player_entity);
                    if let Some(player_health) = player_health {
                        player_health.hp = i32::max(player_health.hp, player_health.max_hp / 2);
                    }
                }

                let new_map = save_map(&mut self.ecs);
                let message = format!("{{\"__MAP__\":\"{}\"}}", new_map).as_bytes().to_vec();
                self.game_client.send_message(message);
                
            } else {
                println!("I am setting map: {}", response[0].1.len());
                load_map(&mut self.ecs, response[0].1.clone());

                let player_entity = self.ecs.fetch::<Entity>();

                // notify the player and give them some health
                let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
                gamelog.entries.push("You descend to the next level, and take a moment to heal.".to_string());
                let mut player_health_store = self.ecs.write_storage::<CombatStats>();
                let player_health = player_health_store.get_mut(*player_entity);
                if let Some(player_health) = player_health {
                    player_health.hp = i32::max(player_health.hp, player_health.max_hp / 2);
                }
            }

            let message = format!("{{\"__TRACK_ME__\":\"{} {}\"}}", self.player_name, current_depth + 1).as_bytes().to_vec();
            self.game_client.send_message(message);

            let message = format!("{{\"__CHANGE__\":\"{} {}\"}}", self.player_name, current_depth).as_bytes().to_vec();
            self.game_client.send_message(message);
        }
    }

    /// delete everything after game over
    fn game_over_cleanup(&mut self) {
        // Delete everything
        let mut to_delete = Vec::new();
        for e in self.ecs.entities().join() {
            to_delete.push(e);
        }
        
        for del in to_delete.iter() {
            self.ecs.delete_entity(*del).expect("Deletion failed");
        }
    }
}


fn main() -> rltk::BError {

    use rltk::RltkBuilder;
    let mut context = RltkBuilder::simple80x50()
        .with_title("Multiplayer Roguelike")
        .build()?;
    context.with_post_scanlines(true);

    let map = Map::new(1);
    let (player_x, player_y) = map.rooms[0].center();

    // initialize game state
    let mut gs = State{ 
        ecs : World::new(),
        game_client : ClientHandler::new(Url::parse("ws://127.0.0.1:6881").expect("Address error")),
        player_name : String::new(),
        enemies : Vec::<String>::new(),
    };

    // register all components
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<InflictDamage>();
    gs.ecs.register::<AreaOfEffect>();
    gs.ecs.register::<Confusion>();
    gs.ecs.register::<SimpleMarker<SerializeMe>>();
    gs.ecs.register::<SerializationHelper>();
    gs.ecs.register::<Equippable>();
    gs.ecs.register::<Equipped>();
    gs.ecs.register::<MeleePowerBonus>();
    gs.ecs.register::<DefenseBonus>();
    gs.ecs.register::<WantsToRemoveItem>();
    gs.ecs.register::<Enemy>();
    
    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());

    // insert player entity to the game state
    let player_entity = spawner::player(&mut gs.ecs, player_x, player_y);
    gs.ecs.insert(rltk::RandomNumberGenerator::new());

    for room in map.rooms.iter().skip(1) {
        spawn_room(&mut gs.ecs, room, 1);
    }

    gs.ecs.insert(map);
    gs.ecs.insert(Point::new(player_x, player_y));
    gs.ecs.insert(player_entity);
    gs.ecs.insert(RunState::EnteringName);
    gs.ecs.insert(GameLog { entries : vec!["Welcome to Rusty Roguelike".to_string()] });

    let message = format!("{{\"__IS_MAP__\":\"{}\"}}", 1).as_bytes().to_vec();
    gs.game_client.send_message(message);

    gs.game_client.get_messages();

    let mut clone = gs.game_client.messages.clone();

    let mut response = clone.into_iter().filter(|(key, _)| *key == "__IS_MAP__").collect::<Vec<_>>();

    while response.is_empty() {
        gs.game_client.get_messages();
        clone = gs.game_client.messages.clone();
        response = clone.into_iter().filter(|(key, _)| *key == "__IS_MAP__").collect::<Vec<_>>();
    }

    // setting map if server has it or send it
    if !response.is_empty() {
        if response[0].1 == "F" {
            let new_map = save_map(&mut gs.ecs);
            let message = format!("{{\"__MAP__\":\"{}\"}}", new_map).as_bytes().to_vec();
            gs.game_client.send_message(message);
            println!("send map");
        } else {
            println!("I am setting map: {}", response[0].1.len());
            load_map(&mut gs.ecs, response[0].1.clone());
        }
    }

    rltk::main_loop(context, gs)
}
