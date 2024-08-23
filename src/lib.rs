pub mod dzi;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Face {
    Back,
    Down,
    Front,
    Left,
    Right,
    Up,
}

impl Face {
    pub fn suffix(&self) -> &'static str {
        match *self {
            Face::Back => "b",
            Face::Down => "d",
            Face::Front => "f",
            Face::Left => "l",
            Face::Right => "r",
            Face::Up => "u",
        }
    }
}