use eyre::Result;
use std::sync::Arc;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crossterm::event::{Event as TerminalEvent, KeyCode, KeyModifiers};

use super::{
    client::S3Client,
    frontend::EventAction,
    view_model::{S3ItemsViewModel, S3Output},
    S3Item, S3ItemType,
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

impl Opt {
    // s3_path uri 를 String을 bucket, prefix 로 빼낸다
    fn parse_s3_path(&self) -> Result<Option<(String, String)>> {
        if let Some(s3_path) = &self.s3_path {
            if let Some(str) = s3_path.strip_prefix("s3://") {
                if let Some(i) = str.find("/") {
                    let bucket = str[..i].to_owned();
                    let prefix = str
                        .strip_prefix(&(bucket.clone() + "/"))
                        .unwrap()
                        .to_owned();

                    // key must be removed
                    let prefix = if !prefix.ends_with("/") {
                        if let Some(i) = prefix.rfind("/") {
                            prefix[..i + 1].to_owned()
                        } else {
                            String::default()
                        }
                    } else {
                        prefix
                    };

                    Ok(Some((bucket, prefix)))
                } else {
                    Ok(Some((str.to_owned(), String::default())))
                }
            } else {
                Err(eyre::eyre!("s3_path must start with s3://"))
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s3_path() {
        let make_opt = |s: &str| Opt {
            s3_path: Some(s.to_owned()),
        };

        assert_eq!(
            make_opt("s3://bucket/p1/p2/").parse_s3_path().unwrap(),
            Some(("bucket".to_owned(), "p1/p2/".to_owned()))
        );
        assert_eq!(
            make_opt("s3://bucket/p1/p2/k").parse_s3_path().unwrap(),
            Some(("bucket".to_owned(), "p1/p2/".to_owned()))
        );
        assert_eq!(
            make_opt("s3://bucket").parse_s3_path().unwrap(),
            Some(("bucket".to_owned(), "".to_owned()))
        );
        assert_eq!(
            make_opt("s3://bucket/").parse_s3_path().unwrap(),
            Some(("bucket".to_owned(), "".to_owned()))
        );
        assert_eq!(
            make_opt("s3://bucket/key").parse_s3_path().unwrap(),
            Some(("bucket".to_owned(), "".to_owned()))
        );
    }
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
        let output = if let Some((bucket, prefix)) = opt.parse_s3_path()? {
            S3Output::Objects(
                self.client
                    .lock()
                    .await
                    .list_objects(&bucket, &prefix)
                    .await?,
            )
        } else {
            S3Output::Buckets(self.client.lock().await.list_buckets().await?)
        };
        self.vm.push(output);
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
                    (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                        self.vm.last();
                        EventAction::NeedReDraw
                    }
                    (KeyCode::Down, KeyModifiers::NONE)
                    | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                        self.vm.next();
                        EventAction::NeedReDraw
                    }
                    (KeyCode::Up, KeyModifiers::NONE)
                    | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                        self.vm.previous();
                        EventAction::NeedReDraw
                    }
                    (KeyCode::Enter, KeyModifiers::NONE) => {
                        self.enter().await;
                        EventAction::NeedReDraw
                    }
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => EventAction::Exit,
                    _ => EventAction::NoNeedReDraw,
                },
                TerminalEvent::Resize(_, _) => EventAction::NeedReDraw,
                _ => EventAction::NoNeedReDraw,
            },
        }
    }

    async fn request_bucket_list(&self) {
        let client_copy = self.client.clone();
        let ev_tx_copy = self.ev_tx.clone();
        tokio::spawn(async move {
            if let Ok(output) = client_copy.lock().await.list_buckets().await {
                ev_tx_copy
                    .send(S3Output::Buckets(output))
                    .await
                    .expect("ev_tx_copy send error");
            } else {
                // TODO: error 처리
            }
        });
    }

    async fn request_object_list(&self, bucket: String, prefix: String) {
        let client_copy = self.client.clone();
        let ev_tx_copy = self.ev_tx.clone();

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

    async fn enter(&mut self) {
        let item = self.vm.selected();

        if let Some(s3_item_type) = item.as_ref().map(|i| i.get_type()) {
            if s3_item_type == S3ItemType::Pop {
                if let Some(i) = self.vm.pop() {
                    if self.vm.item_stack.len() == 0 {
                        if let Some((bucket, prefix)) = i.output().bucket_and_prefix() {
                            let mut components: Vec<_> = prefix.split("/").collect();
                            components.pop();
                            components.pop();
                            let mut prefix = components.join("/");
                            if prefix.is_empty() {
                                self.request_bucket_list().await;
                            } else {
                                if !prefix.ends_with("/") {
                                    prefix.push('/')
                                }
                                self.request_object_list(bucket, prefix).await;
                            }
                        }
                    }
                }
                return;
            }
        }

        let bucket_and_prefix = match item {
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
            self.request_object_list(bucket, prefix).await;
        }
    }
}
