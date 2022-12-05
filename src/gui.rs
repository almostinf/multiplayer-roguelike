use std::borrow::Borrow;
use std::thread;
use std::time::Duration;
use std::cmp;

use rltk::{RGB, Rltk, Point, VirtualKeyCode};
use specs::prelude::*;
use crate::{RunState, Equipped};

use crate::{CombatStats, Player, GameLog, Map, Name, Position, xy_idx, State, InBackpack, Viewshed};

pub fn draw_ui(ecs : &World, ctx : &mut Rltk) {
    ctx.draw_box(0, 43, 79, 6, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK));

    let map = ecs.fetch::<Map>();
    let depth = format!("Depth: {}", map.depth);
    ctx.print_color(2, 43, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), &depth);
    
    let log = ecs.fetch::<GameLog>();

    let mut y = 44;
    for s in log.entries.iter().rev() {
        if y < 49 {
            ctx.print(2, y, s);
        }
        y += 1;
    }

    // Draw mouse cursor
    let mouse_pos = ctx.mouse_pos();
    ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::MAGENTA));

    draw_tooltips(&ecs, ctx);

    let combat_stats = ecs.read_storage::<CombatStats>();
    let players = ecs.read_storage::<Player>();

    for (_player, stats) in (&players, &combat_stats).join() {
        let health = format!(" HP: {} / {}", stats.hp, stats.max_hp);
        ctx.print_color(12, 43, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), &health);

        ctx.draw_bar_horizontal(28, 43, 51, stats.hp, stats.max_hp, RGB::named(rltk::RED), RGB::named(rltk::BLACK));
    }
}

fn draw_tooltips(ecs : &World, ctx : &mut Rltk) {
    let map = ecs.fetch::<Map>();
    let names = ecs.read_storage::<Name>();
    let positions = ecs.read_storage::<Position>();
    
    let mouse_pos = ctx.mouse_pos();
    if mouse_pos.0 >= map.width || mouse_pos.1 >= map.height {
        return;
    }

    let mut tooltip : Vec<String> = Vec::new();
    for (name, position) in (&names, &positions).join() {
        let idx = xy_idx(position.x, position.y);
        if position.x == mouse_pos.0 && position.y == mouse_pos.1 && map.visible_tiles[idx] {
            tooltip.push(name.name.to_string());
        }
    }

    if !tooltip.is_empty() {
        let mut width : i32 = 0;
        for s in tooltip.iter() {
            if width < s.len() as i32 {
                width = s.len() as i32;
            }
        }
        width += 3;

        if mouse_pos.0 > 40 {
            let arrow_pos = Point::new(mouse_pos.0 - 2, mouse_pos.1);
            let left_x = mouse_pos.0 - width;
            let mut y = mouse_pos.1;
            for s in tooltip.iter() {
                ctx.print_color(left_x, y, RGB::named(rltk::WHITE), RGB::named(rltk::GREY), s);
                let padding = (width - s.len() as i32) - 1;
                for i in 0..padding {
                    ctx.print_color(arrow_pos.x - i, arrow_pos.y, RGB::named(rltk::WHITE), RGB::named(rltk::GREY), &" ".to_string());
                }
                y += 1;
            }
            ctx.print_color(arrow_pos.x, arrow_pos.y, RGB::named(rltk::WHITE),RGB::named(rltk::GREY), &"->".to_string());
        } else {
            let arrow_pos = Point::new(mouse_pos.0 + 1, mouse_pos.1);
            let left_x = mouse_pos.0 + 3;
            let mut y = mouse_pos.1;
            for s in tooltip.iter() {
                ctx.print_color(left_x + 1, y, RGB::named(rltk::WHITE), RGB::named(rltk::GREY), s);
                let padding = (width - s.len() as i32) - 1;
                for i in 0..padding {
                    ctx.print_color(arrow_pos.x + i + 1, arrow_pos.y, RGB::named(rltk::WHITE), RGB::named(rltk::GREY), &" ".to_string());
                }
                y += 1;
            }
            ctx.print_color(arrow_pos.x, arrow_pos.y, RGB::named(rltk::WHITE), RGB::named(rltk::GREY), &"<-".to_string());
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum ItemMenuResult {
    Cancel, 
    NoResponse,
    Selected,
}

pub fn show_inventory(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names).join().filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(15, y - 2, 31, (count + 3) as i32, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK));
    ctx.print_color(18, y - 2, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "Inventory");
    ctx.print_color(18, y + count as i32 + 1, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "ESCAPE to cancel");

    let mut equippable : Vec<Entity> = Vec::new();
    let mut j = 0;
    for (entity, _pack, name) in (&entities, &backpack, &names).join().filter(|item| item.1.owner == *player_entity) {
        ctx.set(17, y, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), rltk::to_cp437('('));
        ctx.set(18, y, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), 97+j as rltk::FontCharType);
        ctx.set(19, y, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), rltk::to_cp437(')'));

        ctx.print(21, y, &name.name.to_string());
        equippable.push(entity);
        y += 1;
        j += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => {
            match key {
                VirtualKeyCode::Escape => {
                    (ItemMenuResult::Cancel, None)
                }
                _ => {
                    let selection = rltk::letter_to_option(key);
                    if selection > -1 && selection < count as i32 {
                        return (ItemMenuResult::Selected, Some(equippable[selection as usize]));
                    }
                    (ItemMenuResult::NoResponse, None)
                }
            }
        }
    }
}

pub fn drop_item_menu(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names).join().filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(15, y-2, 31, (count+3) as i32, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK));
    ctx.print_color(18, y-2, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "Drop Which Item?");
    ctx.print_color(18, y+count as i32+1, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "ESCAPE to cancel");

    let mut equippable : Vec<Entity> = Vec::new();
    let mut j = 0;
    for (entity, _pack, name) in (&entities, &backpack, &names).join().filter(|item| item.1.owner == *player_entity) {
        ctx.set(17, y, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), rltk::to_cp437('('));
        ctx.set(18, y, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), 97+j as rltk::FontCharType);
        ctx.set(19, y, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), rltk::to_cp437(')'));

        ctx.print(21, y, &name.name.to_string());
        equippable.push(entity);
        y += 1;
        j += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => {
            match key {
                VirtualKeyCode::Escape => {
                    (ItemMenuResult::Cancel, None)
                }
                _ => {
                    let selection = rltk::letter_to_option(key);
                    if selection> -1 && selection < count as i32 {
                        return (ItemMenuResult::Selected, Some(equippable[selection as usize]));
                    }
                    (ItemMenuResult::NoResponse, None)
                }
            }
        }
    }
}

pub fn ranged_target(gs : &mut State, ctx : &mut Rltk, range : i32) -> (ItemMenuResult, Option<Point>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let player_pos = gs.ecs.fetch::<Point>();
    let viewsheds = gs.ecs.read_storage::<Viewshed>();

    ctx.print_color(5, 0, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "Select Target:");

    // Highlight available target cells
    let mut available_cells = Vec::new();
    let visible = viewsheds.get(*player_entity);
    if let Some(visible) = visible {
        // We have a viewshed
        for idx in visible.visible_tiles.iter() {
            let distance = rltk::DistanceAlg::Pythagoras.distance2d(*player_pos, *idx);
            if distance <= range as f32 {
                ctx.set_bg(idx.x, idx.y, RGB::named(rltk::BLUE));
                available_cells.push(idx);
            }
        }
    } else {
        return (ItemMenuResult::Cancel, None);
    }

    // Draw mouse cursor
    let mouse_pos = ctx.mouse_pos();
    let mut valid_target = false;
    for idx in available_cells.iter() {
        if idx.x == mouse_pos.0 && idx.y == mouse_pos.1 {
            valid_target = true;
        }
    }
    if valid_target {
        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::CYAN));
        if ctx.left_click {
            return (ItemMenuResult::Selected, Some(Point::new(mouse_pos.0, mouse_pos.1)))
        }
    } else {
        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::RED));
        if ctx.left_click {
            return (ItemMenuResult::Cancel, None)
        }
    }
    (ItemMenuResult::NoResponse, None)
}

pub fn remove_item_menu(gs : &mut State, ctx : &mut Rltk) ->  (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<Equipped>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names).join().filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(15, y - 2, 31, (count + 3) as i32, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK));
    ctx.print_color(18, y - 2, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "Remove Which Item?");
    ctx.print_color(18, y + count as i32 + 1, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "ESCAPE to cancel");

    let mut equippable : Vec<Entity> = Vec::new();
    let mut j = 0;
    for (entity, _pack, name) in (&entities, &backpack, &names).join() {
        ctx.set(17, y, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), rltk::to_cp437('('));
        ctx.set(18, y, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), 97 + j as rltk::FontCharType);
        ctx.set(19, y, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), rltk::to_cp437(')'));

        ctx.print(21, y, &name.name.to_string());
        equippable.push(entity);
        y += 1;
        j += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => {
            match key {
                VirtualKeyCode::Escape => {
                    (ItemMenuResult::Cancel, None)
                }
                _ => {
                    let selection = rltk::letter_to_option(key);
                    if selection > -1 && selection < count as i32 {
                        return (ItemMenuResult::Selected, Some(equippable[selection as usize]));
                    }
                    (ItemMenuResult::NoResponse, None)
                }
            }
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuSelection {
    Play,
    SaveGame,
    LoadGame,
    Rating,
    Quit,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuResult {
    NoSelection {
        selected : MainMenuSelection,
    },
    Selected {
        selected : MainMenuSelection,
    }
}

pub fn main_menu(gs : &mut State, ctx : &mut Rltk) -> MainMenuResult {
    let runstate = gs.ecs.fetch::<RunState>();

    ctx.print_color_centered(15, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "Mutliplayer Roguelike");

    if let RunState::MainMenu { menu_selection : selection } = *runstate {
        if selection == MainMenuSelection::Play {
            ctx.print_color_centered(24, RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK), "Play");
        } else {
            ctx.print_color_centered(24, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), "Play");
        }
        
        if selection == MainMenuSelection::SaveGame {
            ctx.print_color_centered(26, RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK), "Save Game");
        } else {
            ctx.print_color_centered(26, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), "Save Game");
        }

        if selection == MainMenuSelection::LoadGame {
            ctx.print_color_centered(28, RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK), "Load Game");
        } else {
            ctx.print_color_centered(28, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), "Load Game");
        }

        if selection == MainMenuSelection::Rating {
            ctx.print_color_centered(30, RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK), "Rating");
        } else {
            ctx.print_color_centered(30, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), "Rating");
        }

        if selection == MainMenuSelection::Quit {
            ctx.print_color_centered(32, RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK), "Quit");
        } else {
            ctx.print_color_centered(32, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), "Quit");
        }

        match ctx.key {
            None => return MainMenuResult::NoSelection { selected: selection },
            Some(key) => {
                match key {
                    VirtualKeyCode::Escape => return MainMenuResult::NoSelection {selected : selection},
                    VirtualKeyCode::Up => {
                        let newselection;
                        match selection {
                            MainMenuSelection::Play => newselection = MainMenuSelection::Quit,
                            MainMenuSelection::SaveGame => newselection = MainMenuSelection::Play,
                            MainMenuSelection::LoadGame => newselection = MainMenuSelection::SaveGame,
                            MainMenuSelection::Rating => newselection = MainMenuSelection::LoadGame,
                            MainMenuSelection::Quit => newselection = MainMenuSelection::Rating,
                        }
                        return MainMenuResult::NoSelection { selected : newselection }
                    }
                    VirtualKeyCode::Down => {
                        let newselection;
                        match selection {
                            MainMenuSelection::Play => newselection = MainMenuSelection::SaveGame,
                            MainMenuSelection::SaveGame => newselection = MainMenuSelection::LoadGame,
                            MainMenuSelection::LoadGame => newselection = MainMenuSelection::Rating,
                            MainMenuSelection::Rating => newselection = MainMenuSelection::Quit,
                            MainMenuSelection::Quit => newselection = MainMenuSelection::Play,
                        }
                        return MainMenuResult::NoSelection { selected : newselection }
                    }
                    VirtualKeyCode::Return => return MainMenuResult::Selected { selected : selection },
                    _ => return MainMenuResult::NoSelection { selected : selection }
                }
            }
        }
    }
    MainMenuResult::NoSelection { selected : MainMenuSelection::Play }
}

#[derive(PartialEq, Clone, Copy)]
pub enum GameOverResult {
    NoSelection, 
    QuitToMenu,
}

pub fn game_over(ctx : &mut Rltk) -> GameOverResult {
    ctx.print_color_centered(15, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "Your journey has ended!");
    ctx.print_color_centered(17, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), "One day, we'll tell you all about how you did.");
    ctx.print_color_centered(18, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), "That day, sadly, is not in this chapter..");

    ctx.print_color_centered(20, RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK), "Press any key to return to the menu");

    thread::sleep(Duration::from_millis(200));

    match ctx.key {
        None => GameOverResult::NoSelection,
        Some(_) => GameOverResult::QuitToMenu,
    }
}

pub fn entering_name<'a>(ctx : &mut Rltk, name : &'a mut String) -> Result<&'a String, ()> {

    ctx.print_color_centered(5, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "Please Enter your name");

    match ctx.key {
        Some(key) => {
            match key {
                VirtualKeyCode::A => (*name).push('a'),
                VirtualKeyCode::B => (*name).push('b'),
                VirtualKeyCode::C => (*name).push('c'),
                VirtualKeyCode::D => (*name).push('d'),
                VirtualKeyCode::E => (*name).push('e'),
                VirtualKeyCode::F => (*name).push('g'),
                VirtualKeyCode::G => (*name).push('g'),
                VirtualKeyCode::H => (*name).push('h'),
                VirtualKeyCode::I => (*name).push('i'),
                VirtualKeyCode::J => (*name).push('j'),
                VirtualKeyCode::K => (*name).push('k'),
                VirtualKeyCode::L => (*name).push('l'),
                VirtualKeyCode::M => (*name).push('m'),
                VirtualKeyCode::N => (*name).push('n'),
                VirtualKeyCode::O => (*name).push('o'),
                VirtualKeyCode::P => (*name).push('p'),
                VirtualKeyCode::Q => (*name).push('q'),
                VirtualKeyCode::R => (*name).push('r'),
                VirtualKeyCode::S => (*name).push('s'),
                VirtualKeyCode::T => (*name).push('t'),
                VirtualKeyCode::U => (*name).push('u'),
                VirtualKeyCode::V => (*name).push('v'),
                VirtualKeyCode::W => (*name).push('w'),
                VirtualKeyCode::X => (*name).push('x'),
                VirtualKeyCode::Y => (*name).push('y'),
                VirtualKeyCode::Z => (*name).push('z'),
                VirtualKeyCode::Back => {
                    if !name.is_empty() {
                        name.pop();
                    }
                }
                VirtualKeyCode::Space => return Ok(name),
                _ => ctx.print_color_centered(12, RGB::named(rltk::RED), RGB::named(rltk::BLACK), "Error!"),
            }
        }
        None => (),
    }
    ctx.print_color_centered(7, RGB::named(rltk::WHITESMOKE), RGB::named(rltk::BLACK), format!("Your current name: {}", name));
    ctx.print_color_centered(9, RGB::named(rltk::ORANGE), RGB::named(rltk::BLACK), "Esc to save");

    Err(())
}

// Need for testing
pub fn show_rating(gs: &mut State, ctx: &mut Rltk) -> ItemMenuResult {
    let message = b"{\"__RATING__\":\"\"}".to_vec();
    gs.game_client.send_message(message);

    let clone = gs.game_client.messages.clone();

    let response = clone.into_iter().filter(|(key, _)| *key == "__RATING__").collect::<Vec<_>>();

    println!("name resp size: {}", response.len());
    
    let mut r = Vec::<(String, i32)>::new();

    if !response.is_empty() {
        let value = &response[0].1;
        let split = value.split(' ');
        let v = split.collect::<Vec<&str>>();
        for record in v {
            let split = record.split(':');
            let v = split.collect::<Vec<&str>>();
            if v.len() > 1 {
                let value = v[1].parse::<i32>().expect("Can't convert to number");
                r.push((v[0].to_string(), value));
            }
        }
    }
    
    // sorting vector by second value from high to low (level)
    r.sort_by_key(|k| cmp::Reverse(k.1));

    let count = r.len();

    let y = (25 - (count / 2)) as i32;
    ctx.draw_box(15, y - 2, 31, (count + 3) as i32, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK));
    ctx.print_color(18, y - 2, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "Rating");
    ctx.print_color(18, y + count as i32 + 1, RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK), "ESCAPE to cancel");

    let mut x = 20;
    for record in r {
        ctx.print_color(x, y, RGB::named(rltk::AQUA), RGB::named(rltk::BLACK), format!("{}: {}", record.0, record.1));
        x += 1;
    }

    match ctx.key {
        None => ItemMenuResult::NoResponse,
        Some(key) => {
            match key {
                VirtualKeyCode::Escape => ItemMenuResult::Cancel,
                _ => ItemMenuResult::NoResponse,
            }
        }
    }
}
