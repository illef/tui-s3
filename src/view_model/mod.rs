use std::sync::{Arc, Mutex};

use aws_sdk_s3::output::ListObjectsOutput;
use tui::widgets::{List, ListState};

use crate::{s3::BucketWithLocation, S3Item};

pub mod ui_converter;

#[derive(Debug, PartialEq)]
pub enum S3OutputType {
    Buckets,
    Objects,
}

pub enum S3Output {
    Buckets(Vec<BucketWithLocation>),
    Objects(ListObjectsOutput),
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
    items: StatefulList<S3Item>,
    output: S3Output,
}

impl S3ItemViewModel {
    fn items(&self) -> &StatefulList<S3Item> {
        &self.items
    }

    fn make_s3_item_from_buckets(output: &Vec<BucketWithLocation>) -> Vec<S3Item> {
        output
            .iter()
            .map(|b| S3Item::Bucket(b.to_owned()))
            .collect()
    }

    fn make_s3_item_from_objects(output: &ListObjectsOutput) -> Vec<S3Item> {
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
            items: StatefulList::new(Self::make_s3_item_from_output(&s3_output)),
            output: s3_output,
        }
    }

    pub fn update_output(&mut self, s3_output: S3Output) {
        assert_eq!(self.output.output_type(), s3_output.output_type());
        self.items
            .update(Self::make_s3_item_from_output(&s3_output));
        self.output = s3_output;
    }

    pub fn output(&self) -> &S3Output {
        &self.output
    }

    pub fn selected(&self) -> Option<&S3Item> {
        self.items.selected()
    }
}

pub struct S3ItemsViewModel {
    item_stack: Vec<S3ItemViewModel>,
}

impl S3ItemsViewModel {
    pub fn new() -> Self {
        Self { item_stack: vec![] }
    }

    pub fn make_view(&self) -> Option<(List<'static>, Arc<Mutex<ListState>>)> {
        self.item_stack.last().map(|i| i.into())
    }

    pub fn selected_s3_uri(&self) -> String {
        match self.selected() {
            Some(S3Item::Bucket(b)) => format!("s3://{}", b.bucket.name().as_ref().unwrap_or(&"")),
            Some(S3Item::Directory(d)) => {
                if let Some((bucket, _)) = self.bucket_and_prefix() {
                    format!("s3://{}/{}", bucket, d.prefix().as_ref().unwrap_or(&""))
                } else {
                    String::default()
                }
            }
            Some(S3Item::Key(k)) => {
                if let Some((bucket, _)) = self.bucket_and_prefix() {
                    format!("s3://{}/{}", bucket, k.key().as_ref().unwrap_or(&""))
                } else {
                    String::default()
                }
            }
            _ => String::default(),
        }
    }

    pub fn next(&mut self) {
        if let Some(i) = self.item_stack.last_mut() {
            i.items.next();
        }
    }

    pub fn previous(&mut self) {
        if let Some(i) = self.item_stack.last_mut() {
            i.items.previous();
        }
    }

    pub fn pop(&mut self) {
        self.item_stack.pop();
    }

    pub fn push(&mut self, s3_output: S3Output) {
        self.item_stack.push(S3ItemViewModel::new(s3_output));
    }

    // 현재 보여지는 값을 전달된 s3_output으로 update한다
    pub fn update(&mut self, s3_output: S3Output) {
        let bucket_and_prefix = self.bucket_and_prefix();
        if let Some(item) = self.item_stack.last_mut() {
            if s3_output.bucket_and_prefix() == bucket_and_prefix {
                item.update_output(s3_output);
            }
        } else {
            self.push(s3_output);
        }
    }

    pub fn selected(&self) -> Option<&S3Item> {
        self.item_stack.last().map(|i| i.selected()).flatten()
    }

    pub fn bucket_and_prefix(&self) -> Option<(String, String)> {
        self.item_stack
            .last()
            .map(|i| i.output().bucket_and_prefix())
            .flatten()
    }
}

struct StatefulList<T> {
    state: Arc<Mutex<ListState>>,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn state(&self) -> Arc<Mutex<ListState>> {
        self.state.clone()
    }
    pub fn items(&self) -> &Vec<T> {
        &self.items
    }

    fn new(items: Vec<T>) -> Self {
        let mut s = StatefulList {
            state: Default::default(),
            items,
        };
        s.next();
        s
    }

    fn selected(&self) -> Option<&T> {
        self.state
            .lock()
            .expect("state lock fail")
            .selected()
            .map(|i| &self.items[i])
    }

    fn update(&mut self, items: Vec<T>) {
        self.items = items;
        let mut state = self.state.lock().expect("state lock fail");
        if let Some(i) = state.selected() {
            if i >= self.items.len() {
                state.select(Some(self.items.len() - 1));
            }
        }
    }

    fn next(&mut self) {
        let mut state = self.state.lock().expect("state lock fail");
        if self.items.len() == 0 {
            self.state.lock().expect("state lock fail").select(None);
        } else {
            let i = match state.selected() {
                Some(i) => {
                    if i >= self.items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            state.select(Some(i));
        }
    }

    fn previous(&mut self) {
        let mut state = self.state.lock().expect("state lock fail");
        if self.items.len() == 0 {
            state.select(None);
        } else {
            let i = match state.selected() {
                Some(i) => {
                    if i > 0 {
                        i - 1
                    } else {
                        0
                    }
                }
                None => 0,
            };
            state.select(Some(i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::S3Item;

    use aws_sdk_s3::{
        model::{Bucket, BucketLocationConstraint, Object},
        output::ListObjectsOutput,
    };

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

            ListObjectsOutput::builder()
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

        assert_eq!(vm.selected(), Some(&S3Item::Key(expect_selected)));
    }
}
