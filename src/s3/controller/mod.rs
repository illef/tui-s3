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

use super::{
    client::S3Client,
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

pub struct Controller {
    // 컨트롤  대상
    vm: Arc<Mutex<S3ItemsViewModel>>,
    client: Arc<Mutex<S3Client>>,
    // UI를 다시 그릴것을 요청하기 위한 sender
    ev_tx: Sender<()>,
    // 후에 UI 쓰레드가 이 Receiver를 가져가게 된다
    ev_rx: Option<Receiver<()>>,
}

impl Controller {
    pub async fn new(opt: Opt) -> Result<Self> {
        let (ev_tx, ev_rx) = channel(100);
        let mut controller = Self {
            vm: Arc::new(Mutex::new(S3ItemsViewModel::new())),
            // TODO: 에러 처리
            client: Arc::new(Mutex::new(S3Client::new().await?)),
            ev_tx,
            ev_rx: Some(ev_rx),
        };

        controller.init(opt).await?;

        Ok(controller)
    }

    pub async fn draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let selected_s3_uri = self.view_model().lock().await.selected_s3_uri();
        let bucket_and_prefix = self.view_model().lock().await.bucket_and_prefix();
        if let Some((widget, state)) = self.view_model().lock().await.make_view() {
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
                    widget,
                    chunks[1], //list_view
                    &mut state.lock().expect("state lock fail"),
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
        }
        Ok(())
    }

    pub fn take_event_receiver(&mut self) -> Receiver<()> {
        self.ev_rx.take().unwrap()
    }

    pub fn view_model(&self) -> Arc<Mutex<S3ItemsViewModel>> {
        self.vm.clone()
    }

    pub async fn previous(&mut self) {
        self.vm.lock().await.previous();
    }

    pub async fn next(&mut self) {
        self.vm.lock().await.next();
    }

    pub async fn init(&mut self, opt: Opt) -> Result<()> {
        let output = self.client.lock().await.list_buckets().await?;
        self.vm.lock().await.update(S3Output::Buckets(output));
        self.ev_tx.send(()).await.expect("ev_tx_copy send error");
        Ok(())
    }

    pub async fn refresh(&mut self) {
        let bucket_and_prefix = { self.vm.lock().await.bucket_and_prefix() };
        let ev_tx_copy = self.ev_tx.clone();
        let vm_copy = self.vm.clone();
        let client_copy = self.client.clone();
        match bucket_and_prefix {
            None => {
                tokio::spawn(async move {
                    if let Ok(output) = client_copy.lock().await.list_buckets().await {
                        vm_copy.lock().await.update(S3Output::Buckets(output));
                        ev_tx_copy.send(()).await.expect("ev_tx_copy send error");
                    } else {
                        // TODO: error 처리
                    }
                });
            }
            Some((bucket, prefix)) => {
                tokio::spawn(async move {
                    if let Ok(output) = client_copy
                        .lock()
                        .await
                        .list_objects(&bucket, &prefix)
                        .await
                    {
                        vm_copy.lock().await.update(S3Output::Objects(output));
                        ev_tx_copy.send(()).await.expect("ev_tx_copy send error");
                    } else {
                        // TODO: error 처리
                    }
                });
            }
        }
    }

    pub async fn enter(&mut self) {
        let item = self.vm.lock().await.selected().map(|i| i.to_owned());
        let ev_tx_copy = self.ev_tx.clone();
        let vm_copy = self.vm.clone();
        let client_copy = self.client.clone();

        let bucket_and_prefix = match item {
            Some(S3Item::Pop) => {
                self.vm.lock().await.pop();
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
                self.vm
                    .lock()
                    .await
                    .bucket_and_prefix()
                    .map(|b| b.0)
                    .unwrap(),
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
                    vm_copy.lock().await.push(S3Output::Objects(output));
                    ev_tx_copy.send(()).await.expect("ev_tx_copy send error");
                } else {
                    // TODO: error 처리
                }
            });
        }
    }
}
