use rltk::{Rltk, GameState, RGB};
use specs::prelude::*;

mod map;
pub use map::*;
mod rect;
pub use rect::Rect;
mod player;
use player::*;
mod components;
pub use components::*;
mod visibility_system;
pub use visibility_system::*;

pub struct State {
    pub ecs : World,
}

impl GameState for State {
    fn tick(&mut self, ctx : &mut Rltk) {
        ctx.cls();

        player_input(self, ctx);
        self.run_systems();

        draw_map(&self.ecs, ctx);
        
        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();

        for (pos, render) in (&positions, &renderables).join() {
            ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
        }
    }
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem{};
        vis.run_now(&self.ecs);
        self.ecs.maintain();
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50()
        .with_title("Roguelike Testing")
        .build()?;
    let mut gs = State{ 
        ecs : World::new(),
    };
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();

    let map = Map::new();
    let (player_x, player_y) = map.rooms[0].center();
    gs.ecs.insert(map); // map is a resource in ecs

    gs.ecs
        .create_entity()
        .with(Position {x : player_x, y : player_y})
        .with(Renderable {
            glyph : rltk::to_cp437('@'),
            fg : RGB::named(rltk::YELLOW),
            bg : RGB::named(rltk::BLACK),
        })
        .with(Player{})
        .with(Viewshed{visible_tiles : Vec::new(), range : 8, dirty : true})
        .build();
    rltk::main_loop(context, gs)
}
