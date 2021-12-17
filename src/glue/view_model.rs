use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{List, ListItem, ListState},
};

use crate::StatefulList;

use super::client::GlueTable;

fn glue_table_as_list_item(table: &GlueTable) -> ListItem {
    ListItem::new(Spans::from(vec![
        Span::styled(
            *table.database.name().as_ref().unwrap_or(&""),
            Style::default().fg(Color::Blue),
        ),
        Span::styled(".", Style::default().fg(Color::Blue)),
        Span::styled(
            *table.table.name().as_ref().unwrap_or(&""),
            Style::default().fg(Color::White),
        ),
    ]))
}

pub struct ViewModel {
    pub list: StatefulList<GlueTable>,
}

impl ViewModel {
    pub fn new(tables: Vec<GlueTable>) -> Self {
        Self {
            list: StatefulList::new(tables),
        }
    }

    pub fn reset_state(&mut self, state: ListState) {
        self.list.state = state;
    }

    pub fn make_list<'a>(&'a self) -> (List<'a>, ListState) {
        let list_state = self.list.state();
        let glue_tables = self.list.items();

        let list_items: Vec<_> = glue_tables
            .iter()
            .map(|gt| glue_table_as_list_item(gt))
            .collect();

        let items = List::new(list_items)
            .highlight_style(
                Style::default()
                    .bg(Color::Green)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("");

        (items, list_state.clone())
    }
}
