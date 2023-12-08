use crossterm::event::Event;
use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

pub trait UIApp {
    fn set_scene(&mut self, scene: impl UIScene);
}

pub trait UIScene {
    fn should_quit(&self) -> bool;

    fn draw(&mut self, frame: &mut Frame, area: Rect);
    fn handle_event(&mut self, event: &Event);
}

pub trait UIWidget<'a> {
    type SharedState;

    fn size(&self, shared_state: &Self::SharedState) -> Constraint;
    fn draw(&mut self, shared_state: &Self::SharedState, frame: &mut Frame, area: Rect);
}

pub fn draw_ui_widgets<S>(
    widgets: &mut [&mut dyn UIWidget<SharedState = S>],
    state: &S,
    frame: &mut Frame,
    area: Rect,
) {
    //Calculate the layout
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            widgets
                .iter()
                .map(|w| w.size(state))
                .chain(std::iter::once(Constraint::Min(0)))
                .collect::<Vec<_>>(),
        )
        .split(area);

    //Draw widgets
    for (i, widget) in widgets.iter_mut().enumerate() {
        widget.draw(state, frame, layout[i]);
    }
}

pub fn draw_ui_widget_border(title: &str, frame: &mut Frame, area: Rect) {
    frame.render_widget(Block::default().borders(Borders::all()).title(title), area);
}
