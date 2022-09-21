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

// #[derive(Component)]
// struct LeftMover {}

// struct LeftWalker {}

// impl<'a> System<'a> for LeftWalker {
//     type SystemData = (ReadStorage<'a, LeftMover>, 
//                         WriteStorage<'a, Position>);
//     fn run(&mut self, (lefty, mut pos) : Self::SystemData) {
//         for (_lefty, pos) in (&lefty, &mut pos).join() {
//             pos.x -= 1;
//             if pos.x < 0 {
//                 pos.x = 79;
//             }
//         }
//     }
// }

pub struct State {
    pub ecs : World,
}

impl GameState for State {
    fn tick(&mut self, ctx : &mut Rltk) {
        ctx.cls();

        player_input(self, ctx);
        self.run_systems();

        let map = self.ecs.fetch::<Vec<TileType>>();
        draw_map(&map, ctx);
        
        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();

        for (pos, render) in (&positions, &renderables).join() {
            ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
        }
    }
}

impl State {
    fn run_systems(&mut self) {
        // let mut lw = LeftWalker{};
        // lw.run_now(&self.ecs);
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
    //gs.ecs.register::<LeftMover>();
    gs.ecs.register::<Player>();

    let (rooms, map) = new_map_rooms_and_corridors();
    gs.ecs.insert(map); // map is a resource in ecs

    let (player_x, player_y) = rooms[0].center();

    gs.ecs
        .create_entity()
        .with(Position {x : player_x, y : player_y})
        .with(Renderable {
            glyph : rltk::to_cp437('@'),
            fg : RGB::named(rltk::YELLOW),
            bg : RGB::named(rltk::BLACK),
        })
        .with(Player{})
        .build();

    // for i in 0..10 {
    //     gs.ecs
    //         .create_entity()
    //         .with(Position {x : i * 7, y : 20})
    //         .with(Renderable {
    //             glyph : rltk::to_cp437('*'),
    //             fg : RGB::named(rltk::RED),
    //             bg : RGB::named(rltk::BLACK),
    //         })
    //         .with(LeftMover{})
    //         .build();
    // }
    rltk::main_loop(context, gs)
}
