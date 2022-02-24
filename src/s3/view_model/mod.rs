use aws_sdk_s3::output::ListObjectsV2Output;
use tui::{
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, List, ListState, Paragraph},
};

pub use super::*;
use super::{client::BucketWithLocation, S3Item};

pub mod ui_converter;
use crate::StatefulList;

#[derive(Debug, PartialEq)]
pub enum S3OutputType {
    Buckets,
    Objects,
}

#[derive(Debug)]
pub enum S3Output {
    Buckets(Vec<BucketWithLocation>),
    Objects(ListObjectsV2Output),
}

impl S3Output {
    fn output_type(&self) -> S3OutputType {
        match self {
            &S3Output::Buckets(_) => S3OutputType::Buckets,
            &S3Output::Objects(_) => S3OutputType::Objects,
        }
    }

    pub fn bucket_and_prefix(&self) -> Option<(String, String)> {
        match &self {
            S3Output::Buckets(_) => None,
            S3Output::Objects(o) => Some((
                o.name().map(|o| o.to_owned()).unwrap_or(String::default()),
                o.prefix()
                    .map(|o| o.to_owned())
                    .unwrap_or(String::default()),
            )),
        }
    }
}

pub struct S3ItemViewModel {
    list: StatefulList<S3Item>,
    output: S3Output,
}

impl S3ItemViewModel {
    fn items(&self) -> &StatefulList<S3Item> {
        &self.list
    }

    fn make_s3_item_from_buckets(output: &Vec<BucketWithLocation>) -> Vec<S3Item> {
        output
            .iter()
            .map(|b| S3Item::Bucket(b.to_owned()))
            .collect()
    }

    fn make_s3_item_from_objects(output: &ListObjectsV2Output) -> Vec<S3Item> {
        std::iter::once(S3Item::Pop)
            .chain(
                output
                    .common_prefixes()
                    .unwrap_or_default()
                    .iter()
                    .map(|p| S3Item::from(p.clone())),
            )
            .chain(
                output
                    .contents()
                    .unwrap_or_default()
                    .iter()
                    .map(|p| S3Item::from(p.clone())),
            )
            .collect()
    }

    fn make_s3_item_from_output(s3_output: &S3Output) -> Vec<S3Item> {
        match s3_output {
            S3Output::Buckets(output) => Self::make_s3_item_from_buckets(&output),
            S3Output::Objects(output) => Self::make_s3_item_from_objects(&output),
        }
    }

    pub fn new(s3_output: S3Output) -> Self {
        Self {
            list: StatefulList::new(Self::make_s3_item_from_output(&s3_output)),
            output: s3_output,
        }
    }

    pub fn search_next(&mut self, search_text: &str) {
        let matched_indexes: Vec<_> = self
            .list
            .items()
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if item.is_matched(search_text) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        let next_matched = matched_indexes
            .iter()
            .filter(|i| **i > self.list.selected_index().unwrap_or_default())
            .next();

        let first_matched = matched_indexes.iter().next();

        if let Some(i) = next_matched.or(first_matched).map(|i| *i) {
            self.list.state.select(Some(i));
        }
    }

    pub fn update_output(&mut self, s3_output: S3Output) {
        assert_eq!(self.output.output_type(), s3_output.output_type());
        self.list.update(Self::make_s3_item_from_output(&s3_output));
        self.output = s3_output;
    }

    pub fn output(&self) -> &S3Output {
        &self.output
    }

    pub fn selected(&self) -> Option<&S3Item> {
        self.list.selected()
    }
}

pub struct S3ItemsViewModel {
    pub list_stack: Vec<S3ItemViewModel>,
}

impl S3ItemsViewModel {
    pub fn new() -> Self {
        Self { list_stack: vec![] }
    }

    pub fn make_selected_s3_item_view(&self) -> Paragraph {
        // selected_s3_uri_view
        let selected_s3_uri_view = Text::from(Span::styled(
            self.selected_s3_uri(),
            Style::default().fg(Color::Black),
        ));

        Paragraph::new(selected_s3_uri_view).style(Style::default().bg(Color::Yellow))
    }

    pub fn make_currenent_common_prefix_view(&self) -> Paragraph {
        let current_search_target = if let Some((bucket, prefix)) = self.bucket_and_prefix() {
            format!("s3://{}/{}    ", bucket, prefix)
        } else {
            "bucket selection    ".to_owned()
        };

        Paragraph::new("")
            .style(Style::default().fg(Color::Cyan))
            .block(
                Block::default()
                    .title(current_search_target)
                    .borders(Borders::BOTTOM),
            )
    }

    pub fn make_item_list_view(&self) -> Option<(List<'static>, ListState)> {
        self.list_stack.last().map(|i| i.into())
    }

    pub fn search_next(&mut self, search_text: &str) {
        if let Some(item) = self.list_stack.last_mut() {
            item.search_next(search_text);
        }
    }

    pub fn reset_state(&mut self, state: ListState) {
        if let Some(item) = self.list_stack.last_mut() {
            item.list.state = state;
        }
    }

    pub fn selected_s3_uri(&self) -> String {
        match self.selected() {
            Some(S3Item::Bucket(b)) => format!("s3://{}", b.bucket.name().as_ref().unwrap_or(&"")),
            Some(S3Item::CommonPrefix(d)) => {
                if let Some((bucket, _)) = self.bucket_and_prefix() {
                    format!("s3://{}/{}", bucket, d.prefix().as_ref().unwrap_or(&""))
                } else {
                    String::default()
                }
            }
            Some(S3Item::Object(k)) => {
                if let Some((bucket, _)) = self.bucket_and_prefix() {
                    format!("s3://{}/{}", bucket, k.key().as_ref().unwrap_or(&""))
                } else {
                    String::default()
                }
            }
            Some(S3Item::Pop) => {
                if let Some((bucket, prefix)) = self.bucket_and_prefix() {
                    format!("s3://{}/{}", bucket, prefix)
                } else {
                    String::default()
                }
            }
            _ => String::default(),
        }
    }

    pub fn last(&mut self) {
        if let Some(i) = self.list_stack.last_mut() {
            if i.list.items.len() > 0 {
                i.list.state.select(Some(i.list.items.len() - 1))
            }
        }
    }

    pub fn first(&mut self) {
        if let Some(i) = self.list_stack.last_mut() {
            if i.list.items.len() > 0 {
                i.list.state.select(Some(0))
            }
        }
    }

    pub fn next(&mut self) {
        if let Some(i) = self.list_stack.last_mut() {
            i.list.next();
        }
    }

    pub fn previous(&mut self) {
        if let Some(i) = self.list_stack.last_mut() {
            i.list.previous();
        }
    }

    pub fn pop(&mut self) -> Option<S3ItemViewModel> {
        self.list_stack.pop()
    }

    pub fn push(&mut self, s3_output: S3Output) {
        self.list_stack.push(S3ItemViewModel::new(s3_output));
    }

    pub fn update(&mut self, s3_output: S3Output) {
        let bucket_and_prefix = self.bucket_and_prefix();
        if let Some(item) = self.list_stack.last_mut() {
            if s3_output.bucket_and_prefix() == bucket_and_prefix {
                item.update_output(s3_output);
            } else {
                self.push(s3_output);
            }
        } else {
            self.push(s3_output);
        }
    }

    pub fn selected(&self) -> Option<&S3Item> {
        self.list_stack.last().map(|i| i.selected()).flatten()
    }

    pub fn bucket_and_prefix(&self) -> Option<(String, String)> {
        self.list_stack
            .last()
            .map(|i| i.output().bucket_and_prefix())
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use aws_sdk_s3::model::{Bucket, BucketLocationConstraint, Object};

    #[test]
    fn test_s3items_view_model() {
        // 처음 상태
        let mut vm = S3ItemsViewModel::new();
        assert_eq!(vm.bucket_and_prefix(), None);
        assert_eq!(vm.selected(), None);

        let bucket_list_output: Vec<BucketWithLocation> = {
            // 버킷 조회 상태
            let bucket_list = vec![
                Bucket::builder().name("bucket1").build(),
                Bucket::builder().name("bucket2").build(),
                Bucket::builder().name("bucket3").build(),
            ];
            let location_list = vec![
                BucketLocationConstraint::ApNortheast1,
                BucketLocationConstraint::ApNortheast1,
                BucketLocationConstraint::ApNortheast1,
            ];

            location_list
                .into_iter()
                .zip(bucket_list.into_iter())
                .map(|(l, b)| BucketWithLocation {
                    location: l,
                    bucket: b,
                })
                .collect()
        };

        // 버킷 조회 결과를 push
        vm.push(S3Output::Buckets(bucket_list_output.clone()));
        assert_eq!(vm.bucket_and_prefix(), None);
        assert_eq!(
            vm.selected(),
            Some(&S3Item::Bucket(bucket_list_output[0].clone()))
        );
        vm.previous();
        assert_eq!(
            vm.selected(),
            Some(&S3Item::Bucket(bucket_list_output[0].clone()))
        );

        vm.next();
        assert_eq!(
            vm.selected(),
            Some(&S3Item::Bucket(bucket_list_output[1].clone()))
        );
        vm.previous();

        assert_eq!(
            vm.selected(),
            Some(&S3Item::Bucket(bucket_list_output[0].clone()))
        );

        let object_list_output = {
            // 버킷, prefix 조회 상태
            let object_list = vec![
                Object::builder().key("obj1").build(),
                Object::builder().key("obj2").build(),
                Object::builder().key("obj3").build(),
            ];

            ListObjectsV2Output::builder()
                .set_contents(Some(object_list))
                .prefix("")
                .build()
        };
        // enter to bucket
        vm.push(S3Output::Objects(object_list_output.clone()));

        assert_eq!(vm.selected(), Some(&S3Item::Pop));
        vm.next();

        let expect_selected = object_list_output
            .clone()
            .contents()
            .map(|b| b[0].to_owned())
            .unwrap();

        assert_eq!(vm.selected(), Some(&S3Item::Object(expect_selected)));
    }
}
