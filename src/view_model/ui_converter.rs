use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{List, ListItem},
};

use super::*;

impl Into<ListItem<'static>> for &S3Item {
    fn into(self) -> ListItem<'static> {
        let span = match self {
            S3Item::Directory(d) => Spans::from(Span::styled(
                d.prefix().unwrap_or("").to_owned(),
                Style::default(),
            )),
            S3Item::Key(k) => Spans::from(Span::styled(
                k.key().unwrap_or("").to_owned(),
                Style::default(),
            )),
            S3Item::Bucket(bucket_with_location) => Spans::from(vec![
                Span::styled(
                    bucket_with_location.location.as_str().to_owned(),
                    Style::default(),
                ),
                Span::styled(
                    bucket_with_location.bucket.name().unwrap_or("").to_owned(),
                    Style::default(),
                ),
            ]),
            S3Item::Pop => Spans::from(Span::styled("..", Style::default())),
        };

        ListItem::new(span).style(Style::default().fg(Color::White).bg(Color::Black))
    }
}

impl Into<(List<'static>, Arc<Mutex<ListState>>)> for &S3ItemViewModel {
    fn into(self) -> (List<'static>, Arc<Mutex<ListState>>) {
        let list_state = self.items().state();
        let list_items: Vec<ListItem> = self.items().items().iter().map(|i| i.into()).collect();
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
