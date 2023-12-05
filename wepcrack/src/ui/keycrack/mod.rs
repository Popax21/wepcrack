pub mod cracker_thread;
pub mod overview;
pub mod scene;
pub mod sigma_info;

use cracker_thread::*;
use overview::*;
pub use scene::*;
use sigma_info::*;

use ratatui::{
    prelude::{Constraint, Rect},
    Frame,
};

trait KeyCrackWidget {
    fn size(&self) -> Constraint;
    fn draw(&mut self, cracker_data: &KeyCrackerThreadData, frame: &mut Frame, area: Rect);
}
