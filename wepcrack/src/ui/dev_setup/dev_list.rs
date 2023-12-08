use crossterm::event::{Event, KeyCode};
use ratatui::{
    prelude::{Constraint, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{HighlightSpacing, List, ListItem, ListState},
};

use crate::ui::{draw_ui_widget_border, UIWidget};

use super::dev_manager::DevManager;

pub(super) struct DevListWidget {
    whipy_list_state: ListState,
}

impl DevListWidget {
    pub fn new() -> DevListWidget {
        DevListWidget {
            whipy_list_state: ListState::default().with_selected(Some(0)),
        }
    }
}

impl DevListWidget {
    pub fn handle_event(&mut self, dev_manager: &DevManager, event: &Event) {
        if let Event::Key(key) = event {
            //Handle whipy list selection
            if let Some(selected) = self.whipy_list_state.selected() {
                let mut selected = selected as isize;
                match key.code {
                    KeyCode::Up => {
                        selected -= 1;
                    }
                    KeyCode::Down => {
                        selected += 1;
                    }
                    _ => {}
                };

                self.whipy_list_state.select(Some(
                    selected.rem_euclid(dev_manager.whipys().len() as isize) as usize,
                ));
            }
        }
    }
}

impl UIWidget<'_> for DevListWidget {
    type SharedState = DevManager;

    fn size(&self, dev_manager: &DevManager) -> Constraint {
        Constraint::Length(2 + dev_manager.whipys().len() as u16)
    }

    fn draw(&mut self, dev_manager: &DevManager, frame: &mut ratatui::Frame, area: Rect) {
        draw_ui_widget_border("Device List", frame, area);

        //Calculate the layout
        let layout = Layout::new()
            .margin(1)
            .constraints([Constraint::Length(dev_manager.whipys().len() as u16)])
            .split(area);

        //Draw the whipy list
        let mut whipy_items = Vec::<ListItem>::new();

        for whipy in dev_manager.whipys() {
            whipy_items.push(ListItem::new(Line::from(whipy.name().bold().cyan())));
        }

        frame.render_stateful_widget(
            List::new(whipy_items)
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always),
            layout[0],
            &mut self.whipy_list_state,
        );
    }
}
