use serde::{Serialize, Deserialize};


#[derive(PartialEq, Serialize, Deserialize, Clone, Copy)]
/// Represents a rectangle (room)
pub struct Rect {
    pub x1 : i32,
    pub x2 : i32,
    pub y1 : i32,
    pub y2 : i32,
}

impl Rect {

    /// Create a new rectangle
    pub fn new(x : i32, y : i32, w : i32, h : i32) -> Self {
        Rect {x1 : x, y1 : y, x2 : x + w, y2 : y + h }
    }

    /// Check if the new rectangle intersect other
    pub fn intersect(&self, other : &Rect) -> bool {
        self.x1 <= other.x2 && self.x2 >= other.x1 && self.y1 <= other.y2 && self.y2 >= other.y1
    }

    /// Return a center position of a room
    pub fn center(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }
}
