use crate::{App, CrosstermTerminal, EventAction, FrontendEvent, StatefulList};

use super::{GlueTable, ViewModel};
use async_trait::async_trait;
use eyre::Result;
use tokio::sync::mpsc::Receiver;

use crossterm::event::{Event as TerminalEvent, KeyCode, KeyEvent, KeyModifiers};
use tui::layout::{Constraint, Direction, Layout};

pub struct Controller {
    vm: ViewModel,
    key_events: Vec<KeyEvent>,
}

impl Controller {
    pub fn new(table: Vec<GlueTable>) -> Self {
        Self {
            vm: ViewModel {
                list: StatefulList::new(table),
            },
            key_events: Default::default(),
        }
    }
}

#[async_trait]
impl App for Controller {
    fn draw(&mut self, terminal: &mut CrosstermTerminal) -> Result<()> {
        let (list, mut state) = self.vm.make_list_view();
        let detail_view = self.vm.make_detail_view();
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(f.size());
            f.render_stateful_widget(list, chunks[0], &mut state);
            if let Some(detail_view) = detail_view {
                f.render_widget(detail_view, chunks[1]);
            }
        })?;
        self.vm.reset_state(state);
        Ok(())
    }
    async fn handle_front_event(&mut self, ev_rx: &mut Receiver<FrontendEvent>) -> EventAction {
        if let Some(event) = ev_rx.recv().await {
            match event {
                FrontendEvent::Tick => {
                    self.key_events.clear();
                    EventAction::NoNeedReDraw
                }
                FrontendEvent::TerminalEvent(terminal_event) => match terminal_event {
                    TerminalEvent::Key(key) => {
                        let last_key_event = self.key_events.last().map(|e| e.to_owned());
                        self.key_events.push(key);
                        match (key.code, key.modifiers) {
                            (KeyCode::Char('q'), KeyModifiers::NONE) => EventAction::Exit,
                            (KeyCode::Char('g'), KeyModifiers::NONE) => {
                                if let Some(key) = last_key_event.as_ref() {
                                    if key.code == KeyCode::Char('g')
                                        && key.modifiers == KeyModifiers::NONE
                                    {
                                        self.vm.list.first();
                                        // gg pressed
                                        EventAction::NeedReDraw
                                    } else {
                                        EventAction::NoNeedReDraw
                                    }
                                } else {
                                    EventAction::NoNeedReDraw
                                }
                            }
                            (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                                self.vm.list.last();
                                EventAction::NeedReDraw
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                                self.vm.list.next();
                                EventAction::NeedReDraw
                            }
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                                self.vm.list.previous();
                                EventAction::NeedReDraw
                            }
                            (KeyCode::Char('c'), KeyModifiers::CONTROL) => EventAction::Exit,
                            _ => EventAction::NoNeedReDraw,
                        }
                    }
                    TerminalEvent::Resize(_, _) => EventAction::NeedReDraw,
                    _ => EventAction::NoNeedReDraw,
                },
            }
        } else {
            EventAction::NoNeedReDraw
        }
    }
}
