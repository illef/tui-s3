use async_trait::async_trait;
use eyre::Result;
use std::sync::Arc;
use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crossterm::event::{Event as TerminalEvent, KeyCode, KeyEvent, KeyModifiers};

use crate::{App, CrosstermTerminal, EventAction, FrontendEvent};

use super::{
    client::S3Client,
    view_model::{S3ItemsViewModel, S3Output},
    S3Item, S3ItemType,
};
use copypasta_ext::{prelude::*, x11_fork::ClipboardContext};
use structopt::StructOpt;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};

#[derive(Debug, StructOpt)]
#[structopt(name = "tui-s3", about = "tui for s3")]
pub struct Opt {
    /// Where to write the output: to `stdout` or `file`
    #[structopt(parse(from_str))]
    s3_path: Option<String>,

    #[structopt(parse(from_str), long = "profile")]
    profile: Option<String>,

    #[structopt(parse(from_str), long = "endpoint-url", short = "e")]
    endpoint_url: Option<String>,
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
            profile: None,
            endpoint_url: None,
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
    KeyEvent(FrontendEvent),
}

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Search,
}

pub struct Controller {
    // 컨트롤  대상
    vm: S3ItemsViewModel,
    client: Arc<Mutex<S3Client>>,
    // UI를 다시 그릴것을 요청하기 위한 sender
    ev_tx: Sender<S3Output>,
    ev_rx: Receiver<S3Output>,
    key_events: Vec<KeyEvent>,
    clipboard_context: Arc<Mutex<ClipboardContext>>,
    input_mode: InputMode,
    search_input: String,
}
impl Controller {
    pub async fn new(opt: Opt) -> Result<Self> {
        let (ev_tx, ev_rx) = channel(100);
        let mut controller = Self {
            vm: S3ItemsViewModel::new(),
            // TODO: 에러 처리
            client: Arc::new(Mutex::new(
                S3Client::new(opt.profile.as_ref(), opt.endpoint_url.as_ref()).await?,
            )),
            ev_tx,
            ev_rx,
            key_events: Default::default(),
            clipboard_context: Arc::new(Mutex::new(ClipboardContext::new().unwrap())),
            input_mode: InputMode::Normal,
            search_input: String::default(),
        };

        controller.init(opt).await?;

        Ok(controller)
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

    async fn handle_event_in_nomal_mode(
        &mut self,
        key: KeyEvent,
        last_key_event: Option<KeyEvent>,
    ) -> EventAction {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::NONE) => EventAction::Exit,
            (KeyCode::Char('g'), KeyModifiers::NONE) => {
                if let Some(key) = last_key_event.as_ref() {
                    if key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::NONE {
                        self.vm.first();
                        // gg pressed
                        EventAction::NeedReDraw
                    } else {
                        EventAction::NoNeedReDraw
                    }
                } else {
                    EventAction::NoNeedReDraw
                }
            }
            (KeyCode::Char('y'), KeyModifiers::NONE) => {
                self.clipboard_context
                    .lock()
                    .await
                    .set_contents(self.vm.selected_s3_uri())
                    .unwrap();
                EventAction::NoNeedReDraw
            }
            (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                self.vm.last();
                EventAction::NeedReDraw
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                self.refresh().await;
                EventAction::NoNeedReDraw
            }
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.vm.next();
                EventAction::NeedReDraw
            }
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                self.vm.previous();
                EventAction::NeedReDraw
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.enter().await;
                EventAction::NeedReDraw
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                self.search_next();
                EventAction::NeedReDraw
            }
            (KeyCode::Char('/'), KeyModifiers::NONE) => {
                self.search_input = "/".to_owned();
                self.input_mode = InputMode::Search;
                EventAction::NeedReDraw
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => EventAction::Exit,
            _ => EventAction::NoNeedReDraw,
        }
    }

    async fn handle_event_in_edit_mode(
        &mut self,
        key: KeyEvent,
        _last_key_event: Option<KeyEvent>,
    ) -> EventAction {
        match key.code {
            KeyCode::Backspace => {
                if self.search_input.len() > 1 {
                    self.search_input.pop();
                }
                self.search_next();
                EventAction::NeedReDraw
            }
            KeyCode::Char(c) => {
                self.search_input.push(c);
                self.search_next();
                EventAction::NeedReDraw
            }
            KeyCode::Esc | KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                EventAction::NeedReDraw
            }
            _ => EventAction::NoNeedReDraw,
        }
    }

    async fn handle_event(&mut self, event: Event) -> EventAction {
        match event {
            Event::ClientEvent(s3output) => {
                self.vm.update(s3output);
                EventAction::NeedReDraw
            }
            Event::KeyEvent(key_event) => match key_event {
                FrontendEvent::Tick => {
                    self.key_events.clear();
                    EventAction::NoNeedReDraw
                }
                FrontendEvent::TerminalEvent(terminal_event) => match terminal_event {
                    TerminalEvent::Key(key) => {
                        let last_key_event = self.key_events.last().map(|e| e.to_owned());
                        self.key_events.push(key);
                        if self.input_mode == InputMode::Normal {
                            self.handle_event_in_nomal_mode(key, last_key_event).await
                        } else {
                            self.handle_event_in_edit_mode(key, last_key_event).await
                        }
                    }
                    TerminalEvent::Resize(_, _) => EventAction::NeedReDraw,
                    _ => EventAction::NoNeedReDraw,
                },
            },
        }
    }

    fn search_next(&mut self) {
        if self.search_input.starts_with("/") {
            self.vm.search_next(&self.search_input[1..]);
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

    async fn refresh(&mut self) {
        if let Some((bucket, prefix)) = self.vm.bucket_and_prefix() {
            self.request_object_list(bucket, prefix).await;
        }
    }

    async fn enter(&mut self) {
        let item = self.vm.selected();

        if let Some(s3_item_type) = item.as_ref().map(|i| i.get_type()) {
            if s3_item_type == S3ItemType::Pop {
                if let Some(i) = self.vm.pop() {
                    if self.vm.list_stack.len() == 0 {
                        if let Some((bucket, prefix)) = i.output().bucket_and_prefix() {
                            if prefix.is_empty() {
                                self.request_bucket_list().await;
                                return;
                            }
                            let mut components: Vec<_> = prefix.split("/").collect();
                            components.pop();
                            components.pop();
                            let mut prefix = components.join("/");
                            if !prefix.is_empty() {
                                prefix.push('/');
                            }
                            self.request_object_list(bucket, prefix).await;
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

#[async_trait]
impl App for Controller {
    fn draw(&mut self, terminal: &mut CrosstermTerminal) -> Result<()> {
        let widget_and_state = self.vm.make_item_list_view();
        if let Some((s3_items_view, mut state)) = widget_and_state {
            terminal.draw(|f| {
                let rect = f.size();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Length(rect.height - 3),
                            Constraint::Min(1),
                            Constraint::Min(1),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                f.render_widget(self.vm.make_currenent_common_prefix_view(), chunks[0]);
                f.render_stateful_widget(s3_items_view, chunks[1], &mut state);
                f.render_widget(self.vm.make_selected_s3_item_view(), chunks[2]);

                // search input view
                let search_input_view =
                    Text::from(Span::styled(&self.search_input, Style::default()));

                let paragraph = Paragraph::new(search_input_view).style(Style::default());
                f.render_widget(paragraph, chunks[3]);
            })?;

            self.vm.reset_state(state);
        }
        Ok(())
    }

    async fn handle_front_event(
        &mut self,
        frontenv_event_rx: &mut Receiver<FrontendEvent>,
    ) -> EventAction {
        let event = tokio::select! {
            // Key Code 이벤트 처리
            Some(frontend_event) = frontenv_event_rx.recv() => Event::KeyEvent(frontend_event),
            Some(s3_output) = self.ev_rx.recv() => Event::ClientEvent(s3_output)
        };

        self.handle_event(event).await
    }
}
