use strum::IntoEnumIterator;
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{List, ListItem},
};

use crate::S3ItemType;

use super::*;

fn rows_into_list_item(columns: Vec<(String, String, String)>) -> Vec<ListItem<'static>> {
    if columns.is_empty() {
        return vec![];
    }
    let first_column_hint = columns.iter().map(|t| t.0.len()).max().unwrap();
    let second_column_hint = columns.iter().map(|t| t.1.len()).max().unwrap();

    let get_left_padding = |width_hint, len| {
        std::iter::repeat(" ")
            .take(width_hint - len)
            .fold(String::new(), |f, s| f + s)
    };

    columns
        .into_iter()
        .map(|i| {
            ListItem::new(Spans::from(vec![
                Span::styled(
                    get_left_padding(first_column_hint, i.0.len()) + &i.0 + " ",
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    get_left_padding(second_column_hint, i.1.len()) + &i.1 + " ",
                    Style::default().fg(Color::Blue),
                ),
                Span::styled(i.2, Style::default()),
            ]))
            .style(Style::default().fg(Color::White).bg(Color::Black))
        })
        .collect()
}

impl Into<(List<'static>, Arc<Mutex<ListState>>)> for &S3ItemViewModel {
    fn into(self) -> (List<'static>, Arc<Mutex<ListState>>) {
        let list_state = self.items().state();
        let s3items = self.items().items();

        let list_items: Vec<_> = S3ItemType::iter()
            .map(|t| {
                let vec = s3items
                    .iter()
                    .filter(|i| i.get_type() == t)
                    .map(|s| s.as_row())
                    .collect();
                rows_into_list_item(vec)
            })
            .flatten()
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
