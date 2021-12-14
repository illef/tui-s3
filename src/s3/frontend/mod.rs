use eyre::Result;

use tokio::{sync::mpsc::channel, task::JoinHandle};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, time::Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use super::controller::Controller;

pub async fn run_frontend(controller: Controller) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, controller).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_key_event_sender(
    tx: tokio::sync::mpsc::Sender<Event>,
    exit_receiver: std::sync::mpsc::Receiver<()>,
) -> JoinHandle<Result<()>> {
    tokio::task::spawn_blocking(move || loop {
        if let Ok(_) = exit_receiver.try_recv() {
            return Ok(());
        }
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Ok(event) = event::read() {
                if tx.blocking_send(event).is_err() {
                    return Ok(());
                }
            }
        }
    })
}

enum EventAction {
    NeedReDraw,
    NoNeedReDraw,
    Exit,
}

async fn handle_event(event: Option<Event>, controller: &mut Controller) -> EventAction {
    if let Some(event) = event {
        match event {
            Event::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) => EventAction::Exit,
                (KeyCode::Down, _) => {
                    controller.next().await;
                    EventAction::NeedReDraw
                }
                (KeyCode::Up, _) => {
                    controller.previous().await;
                    EventAction::NeedReDraw
                }
                (KeyCode::Enter, _) => {
                    controller.enter().await;
                    EventAction::NeedReDraw
                }
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => EventAction::Exit,
                _ => EventAction::NoNeedReDraw,
            },
            Event::Resize(_, _) => EventAction::NeedReDraw,
            _ => EventAction::NoNeedReDraw,
        }
    } else {
        EventAction::NoNeedReDraw
    }
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut controller: Controller) -> Result<()> {
    let (tx, mut event_rx) = channel::<Event>(10);

    let mut update_rx = controller.take_event_receiver();
    let (exit_tx, exit_rx) = std::sync::mpsc::channel();

    // crossterm 으로 부터 이벤트를 받는다
    let key_event_sender = run_key_event_sender(tx, exit_rx);

    'ui: loop {
        controller.draw(terminal).await?;
        'event: loop {
            tokio::select! {
                // Key Code 이벤트 처리
                event = event_rx.recv() => {
                    match handle_event(event, &mut controller).await {
                        EventAction::Exit => {
                            exit_tx.send(())?;
                            break 'ui;
                        }
                        EventAction::NeedReDraw => { break 'event; }
                        EventAction::NoNeedReDraw => {}
                    }
                 },
                _ = update_rx.recv() => {
                    break 'event;
                }
            }
        }
    }

    key_event_sender.await??;
    Ok(())
}
