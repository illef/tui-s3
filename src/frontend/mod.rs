use eyre::Result;

use tokio::sync::mpsc::channel;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use crate::controller::S3ItemsViewModelController;

pub async fn run_frontend() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal).await;

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

async fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let (tx, mut event_rx) = channel::<Event>(10);
    let (ev_tx, mut update_rx) = channel::<()>(10);

    let mut controller = S3ItemsViewModelController::new(ev_tx).await?;
    controller.refresh().await;

    // crossterm 으로 부터 이벤트를 받는다
    run_key_event_sender(tx);

    loop {
        if let Some((widget, state)) = controller.view_model().lock().await.make_view() {
            terminal.draw(|f| {
                f.render_stateful_widget(
                    widget,
                    f.size(),
                    &mut state.lock().expect("state lock fail"),
                );
            })?;
        }

        tokio::select! {
            Some(Event::Key(key)) = event_rx.recv() => {
                 match key.code {
                     // 종료
                     KeyCode::Char('q') => {
                        return Ok(());
                     },
                     KeyCode::Down => controller.next().await,
                     KeyCode::Up => controller.previous().await,
                     KeyCode::Enter =>controller.enter().await,
                     _ => {}
                 }
             },
            _ = update_rx.recv() => {}
        }
    }
}
