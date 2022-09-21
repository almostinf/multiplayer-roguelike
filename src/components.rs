use specs::prelude::*;
use specs_derive::*;
use rltk::{RGB};

#[derive(Component)]
pub struct Position {
    pub x : i32,
    pub y : i32,
}
// #[derive(Component)] is the same like:
// impl Component for Position {
//     type Storage = VecStorage<Self>;
// }

#[derive(Component)]
pub struct Renderable {
    pub glyph : rltk::FontCharType,
    pub fg : RGB, // foreground
    pub bg : RGB, // background
}

#[derive(Component, Debug)]
pub struct Player {}