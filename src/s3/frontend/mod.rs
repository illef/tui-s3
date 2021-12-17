use eyre::Result;

use tokio::{sync::mpsc::channel, task::JoinHandle};

use crossterm::{
    event::{self, Event as TerminalEvent},
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
    tx: tokio::sync::mpsc::Sender<FrontendEvent>,
    exit_receiver: std::sync::mpsc::Receiver<()>,
) -> JoinHandle<Result<()>> {
    tokio::task::spawn_blocking(move || loop {
        if let Ok(_) = exit_receiver.try_recv() {
            return Ok(());
        }
        if crossterm::event::poll(Duration::from_millis(500))? {
            if let Ok(event) = event::read() {
                if tx
                    .blocking_send(FrontendEvent::TerminalEvent(event))
                    .is_err()
                {
                    return Ok(());
                }
            }
        } else {
            if tx.blocking_send(FrontendEvent::Tick).is_err() {
                return Ok(());
            }
        }
    })
}

#[derive(Debug)]
pub enum FrontendEvent {
    Tick,
    TerminalEvent(TerminalEvent),
}

pub enum EventAction {
    NeedReDraw,
    NoNeedReDraw,
    Exit,
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut controller: Controller) -> Result<()> {
    let (tx, mut event_rx) = channel::<FrontendEvent>(10);

    let (exit_tx, exit_rx) = std::sync::mpsc::channel();

    // crossterm 으로 부터 key 이벤트를 받는다
    let key_event_sender = run_key_event_sender(tx, exit_rx);

    controller.draw(terminal)?;
    loop {
        match controller.handle_front_event(&mut event_rx).await {
            EventAction::Exit => {
                break;
            }
            EventAction::NeedReDraw => {
                controller.draw(terminal)?;
            }
            _ => {}
        }
    }

    exit_tx.send(())?;
    key_event_sender.await??;
    Ok(())
}
