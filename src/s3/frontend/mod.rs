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

use super::controller::{Controller, Event};

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
    tx: tokio::sync::mpsc::Sender<TerminalEvent>,
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

pub enum EventAction {
    NeedReDraw,
    NoNeedReDraw,
    Exit,
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut controller: Controller) -> Result<()> {
    let (tx, mut event_rx) = channel::<TerminalEvent>(10);

    let mut client_event_rx = controller.take_event_receiver();
    let (exit_tx, exit_rx) = std::sync::mpsc::channel();

    // crossterm 으로 부터 이벤트를 받는다
    let key_event_sender = run_key_event_sender(tx, exit_rx);

    controller.draw(terminal)?;
    loop {
        let event = tokio::select! {
            // Key Code 이벤트 처리
            Some(event) = event_rx.recv() => Event::TerminalEvent(event),
            Some(s3_output) = client_event_rx.recv() => Event::ClientEvent(s3_output)
        };

        match controller.handle_event(event).await {
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
