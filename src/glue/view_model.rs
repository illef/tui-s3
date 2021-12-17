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

    pub fn make_detail_view<'a>(&'a self) -> Option<List<'a>> {
        if let Some(columns) = self
            .list
            .selected()
            .map(|gt| gt.table.storage_descriptor())
            .flatten()
            .map(|st| st.columns())
            .flatten()
        {
            let names: Vec<_> = columns
                .iter()
                .map(|i| *i.name().as_ref().unwrap_or(&""))
                .collect();
            let types: Vec<_> = columns
                .iter()
                .map(|i| *i.r#type().as_ref().unwrap_or(&""))
                .collect();
            if let Some(max_name_len) = names
                .iter()
                .max_by(|x, y| x.len().cmp(&y.len()))
                .map(|max_str| max_str.len())
            {
                let list_items: Vec<_> = names
                    .iter()
                    .zip(types.iter())
                    .map(|(name, type_)| {
                        let padding = std::iter::repeat(" ")
                            .take(max_name_len - name.len())
                            .fold(String::new(), |f, s| f + s);

                        ListItem::new(Spans::from(vec![
                            Span::styled(
                                (*name).to_owned() + &padding,
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(" : ", Style::default().fg(Color::White)),
                            Span::styled(*type_, Style::default().fg(Color::Yellow)),
                        ]))
                    })
                    .collect();

                Some(List::new(list_items))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn make_list_view<'a>(&'a self) -> (List<'a>, ListState) {
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
