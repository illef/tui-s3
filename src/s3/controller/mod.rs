use eyre::Result;
use std::sync::Arc;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};

use crossterm::event::{Event as TerminalEvent, KeyCode, KeyModifiers};

use super::{
    client::S3Client,
    frontend::EventAction,
    view_model::{S3ItemsViewModel, S3Output},
    S3Item,
};
use structopt::StructOpt;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};

#[derive(Debug, StructOpt)]
#[structopt(name = "tui-s3", about = "tui for s3")]
pub struct Opt {
    /// Where to write the output: to `stdout` or `file`
    #[structopt(short, help("s3 path to search"))]
    s3_path: Option<String>,
}

#[derive(Debug)]
pub enum Event {
    ClientEvent(S3Output),
    TerminalEvent(TerminalEvent),
}

pub struct Controller {
    // 컨트롤  대상
    vm: S3ItemsViewModel,
    client: Arc<Mutex<S3Client>>,
    // UI를 다시 그릴것을 요청하기 위한 sender
    ev_tx: Sender<S3Output>,
    // 후에 UI 쓰레드가 이 Receiver를 가져가게 된다
    ev_rx: Option<Receiver<S3Output>>,
}

impl Controller {
    pub async fn new(opt: Opt) -> Result<Self> {
        let (ev_tx, ev_rx) = channel(100);
        let mut controller = Self {
            vm: S3ItemsViewModel::new(),
            // TODO: 에러 처리
            client: Arc::new(Mutex::new(S3Client::new().await?)),
            ev_tx,
            ev_rx: Some(ev_rx),
        };

        controller.init(opt).await?;

        Ok(controller)
    }

    pub fn draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let selected_s3_uri = self.vm.selected_s3_uri();
        let bucket_and_prefix = self.vm.bucket_and_prefix();
        let widget_and_state = self.vm.make_view();
        if let Some((widget, mut state)) = widget_and_state {
            terminal.draw(|f| {
                let rect = f.size();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Length(rect.height - 2),
                            Constraint::Min(1),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                let current_search_target = if let Some((bucket, prefix)) = bucket_and_prefix {
                    format!("s3://{}/{}    ", bucket, prefix)
                } else {
                    "bucket selection    ".to_owned()
                };

                let paragraph = Paragraph::new("")
                    .style(Style::default().fg(Color::Cyan))
                    .block(
                        Block::default()
                            .title(current_search_target)
                            .borders(Borders::BOTTOM),
                    );

                f.render_widget(
                    paragraph, chunks[0], //list_view
                );

                f.render_stateful_widget(
                    widget, chunks[1], //list_view
                    &mut state,
                );

                let lines = Text::from(Span::styled(
                    selected_s3_uri,
                    Style::default().fg(Color::Yellow),
                ));

                let paragraph = Paragraph::new(lines).style(Style::default());
                f.render_widget(
                    paragraph, chunks[2], //list_view
                );
            })?;

            self.vm.reset_state(state);
        }
        Ok(())
    }

    pub fn take_event_receiver(&mut self) -> Receiver<S3Output> {
        self.ev_rx.take().unwrap()
    }

    pub async fn init(&mut self, opt: Opt) -> Result<()> {
        let output = self.client.lock().await.list_buckets().await?;
        self.vm.push(S3Output::Buckets(output));
        Ok(())
    }

    pub async fn handle_event(&mut self, event: Event) -> EventAction {
        match event {
            Event::ClientEvent(s3output) => {
                self.vm.push(s3output);
                EventAction::NeedReDraw
            }
            Event::TerminalEvent(terminal_event) => match terminal_event {
                TerminalEvent::Key(key) => match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::NONE) => EventAction::Exit,
                    (KeyCode::Down, _) => {
                        self.vm.next();
                        EventAction::NeedReDraw
                    }
                    (KeyCode::Up, _) => {
                        self.vm.previous();
                        EventAction::NeedReDraw
                    }
                    (KeyCode::Enter, _) => {
                        self.enter().await;
                        EventAction::NeedReDraw
                    }
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => EventAction::Exit,
                    _ => EventAction::Exit,
                },
                TerminalEvent::Resize(_, _) => EventAction::NeedReDraw,
                _ => EventAction::NoNeedReDraw,
            },
        }
    }

    async fn enter(&mut self) {
        let item = self.vm.selected();
        let ev_tx_copy = self.ev_tx.clone();
        let client_copy = self.client.clone();

        let bucket_and_prefix = match item {
            Some(S3Item::Pop) => {
                self.vm.pop();
                None
            }
            Some(S3Item::Bucket(bucket_with_location)) => Some((
                bucket_with_location
                    .bucket
                    .name()
                    .map(|b| b.to_owned())
                    .unwrap(),
                "".to_owned(),
            )),
            Some(S3Item::CommonPrefix(d)) => Some((
                self.vm.bucket_and_prefix().map(|b| b.0).unwrap(),
                d.prefix().map(|d| d.to_owned()).unwrap(),
            )),
            _ => None,
        };

        if let Some((bucket, prefix)) = bucket_and_prefix {
            tokio::spawn(async move {
                if let Ok(output) = client_copy
                    .lock()
                    .await
                    .list_objects(&bucket, &prefix)
                    .await
                {
                    ev_tx_copy
                        .send(S3Output::Objects(output))
                        .await
                        .expect("ev_tx_copy send error");
                } else {
                    // TODO: error 처리
                }
            });
        }
    }
}
