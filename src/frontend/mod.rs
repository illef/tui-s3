use eyre::Result;

use tokio::sync::{
    mpsc::{channel, Receiver},
    Mutex,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{List, ListItem, ListState},
    Frame, Terminal,
};

use crate::S3Item;
use crate::{RuntimeState, S3ClientEvent};

struct StatefulList {
    state: ListState,
    items: Vec<S3Item>,
}

impl StatefulList {
    fn new(items: Vec<S3Item>) -> StatefulList {
        StatefulList {
            state: Default::default(),
            items,
        }
    }

    fn update(&mut self, items: Vec<S3Item>) {
        self.state = Default::default();
        self.items = items;
    }

    fn next(&mut self) {
        if self.items.len() == 0 {
            self.state.select(None);
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.items.len() - 1 {
                        0
                    } else {
                        i + 1
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
                    if i == 0 {
                        self.items.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }
}

pub async fn run_frontend(
    runtime_state: Arc<Mutex<RuntimeState>>,
    s3_event_receiver: Receiver<S3ClientEvent>,
) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, runtime_state, s3_event_receiver).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_key_event_sender(tx: tokio::sync::mpsc::Sender<Event>) {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(10);

    tokio::task::spawn_blocking(move || loop {
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout).is_ok() {
            if let Ok(event) = event::read() {
                if tx.blocking_send(event).is_err() {
                    break;
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    });
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    runtime_state: Arc<Mutex<RuntimeState>>,
    mut s3_event_receiver: Receiver<S3ClientEvent>,
) -> Result<()> {
    let (tx, mut event_rx) = channel::<Event>(10);

    // crossterm 으로 부터 이벤트를 받는다
    run_key_event_sender(tx);

    let mut stateful_list = {
        let runtime_state = runtime_state.lock().await;
        StatefulList::new(runtime_state.items())
    };

    loop {
        terminal.draw(|f| ui(f, &mut stateful_list))?;

        tokio::select! {
            Some(Event::Key(key)) = event_rx.recv() => {
                 match key.code {
                     KeyCode::Char('q') => return Ok(()),
                     KeyCode::Left => stateful_list.unselect(),
                     KeyCode::Down => stateful_list.next(),
                     KeyCode::Up => stateful_list.previous(),
                     _ => {}
                 }
             },
            _ = s3_event_receiver.recv() => {
                 let rs = runtime_state.lock().await;
                 stateful_list.update(rs.items());

            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut StatefulList) {
    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .items
        .iter()
        .map(|item| {
            let span = match item {
                S3Item::Directory(d) => Spans::from(Span::styled(
                    d.prefix().unwrap_or("").to_owned(),
                    Style::default(),
                )),
                S3Item::Key(k) => Spans::from(Span::styled(
                    k.key().unwrap_or("").to_owned(),
                    Style::default(),
                )),
                S3Item::Bucket(b) => Spans::from(Span::styled(
                    b.name().unwrap_or("").to_owned(),
                    Style::default(),
                )),
            };
            ListItem::new(span).style(Style::default().fg(Color::White).bg(Color::Black))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    // We can now render the item list
    f.render_stateful_widget(items, f.size(), &mut app.state);
}
