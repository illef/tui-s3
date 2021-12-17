use tui::widgets::ListState;

pub mod frontend;
pub mod glue;
pub mod s3;

pub use frontend::*;

pub struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn state(&self) -> ListState {
        self.state.clone()
    }
    pub fn items(&self) -> &Vec<T> {
        &self.items
    }

    fn new(items: Vec<T>) -> Self {
        let mut s = StatefulList {
            state: Default::default(),
            items,
        };
        s.next();
        s
    }

    fn selected(&self) -> Option<&T> {
        self.state.selected().map(|i| &self.items[i])
    }

    fn update(&mut self, items: Vec<T>) {
        self.items = items;
        if let Some(i) = self.state.selected() {
            if i >= self.items.len() {
                self.state.select(Some(self.items.len() - 1));
            }
        }
    }

    fn next(&mut self) {
        if self.items.len() == 0 {
            self.state.select(None);
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i < self.items.len() - 1 {
                        i + 1
                    } else {
                        i
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    fn previous(&mut self) {
        if self.items.len() == 0 {
            self.state.select(None);
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i > 0 {
                        i - 1
                    } else {
                        0
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }
}
