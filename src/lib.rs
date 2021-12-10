use aws_sdk_s3::{
    model::{Bucket, CommonPrefix, Object},
    output::ListObjectsOutput,
};

pub mod frontend;
pub mod s3;

#[derive(Clone, Debug, PartialEq)]
pub enum S3Item {
    Pop, //상위 디렉토리를 가리키는 객체
    Bucket(Bucket),
    Directory(CommonPrefix),
    Key(Object),
}

impl S3Item {
    fn from_list_output(output: &ListObjectsOutput) -> Vec<S3Item> {
        std::iter::once(S3Item::Pop)
            .chain(
                output
                    .common_prefixes()
                    .unwrap_or_default()
                    .iter()
                    .map(|p: &CommonPrefix| S3Item::from(p.clone())),
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
}

impl From<Bucket> for S3Item {
    fn from(bucket: Bucket) -> Self {
        S3Item::Bucket(bucket)
    }
}

impl From<CommonPrefix> for S3Item {
    fn from(common_prefix: CommonPrefix) -> Self {
        S3Item::Directory(common_prefix)
    }
}

impl From<Object> for S3Item {
    fn from(object: Object) -> Self {
        S3Item::Key(object)
    }
}

// S3Client 가 전달하는 작업 이벤트
#[derive(Debug)]
pub enum S3ClientEvent {
    Completed,
    Progessing(String),
}

// S3Client 가 작업을 마친후 생성하는 이벤트
#[derive(Debug)]
pub enum FrontendEvent {
    // 버켓, 또는 directory 내부로 들어가길 요청하는 이벤트
    Enter(S3Item),
    // S3Storage를 refresh하라는 이벤트
    Refesh,
    End,
}

// UI는 RuntimeState를 화면에 그리면 된다
pub struct RuntimeState {
    // 현재 선택된 버켓
    bucket: Option<String>,
    // 현재 조회하고 있는 prefix
    prefix: String,

    // 현재 화면에 보여줄 Item
    items: Vec<S3Item>,
}

impl RuntimeState {
    pub fn new() -> RuntimeState {
        RuntimeState {
            bucket: Default::default(),
            prefix: Default::default(),
            items: Default::default(),
        }
    }

    pub fn items(&self) -> Vec<S3Item> {
        self.items.clone()
    }

    pub fn bucket(&self) -> Option<String> {
        self.bucket.clone()
    }

    pub fn set_bucket(&mut self, bucket: String) {
        self.bucket = Some(bucket);
    }

    pub fn set_items(&mut self, items: Vec<S3Item>) {
        self.items = items;
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn set_prefix(&mut self, prefix: &str) {
        self.prefix = prefix.into()
    }

    pub fn pop_prefix(&self) -> String {
        if !self.prefix.is_empty() {
            assert!(self.prefix.ends_with("/"));
            let mut split = self.prefix.split("/").collect::<Vec<_>>();
            split.pop(); // last member is empty string
            split.pop(); // delete last component
            if split.len() > 0 {
                return split.join("/") + "/";
            }
        }
        String::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_state_prefix() {
        let mut runtime_state = RuntimeState::new();
        assert_eq!(runtime_state.prefix, "");
        runtime_state.set_prefix("test/");
        assert_eq!(runtime_state.prefix, "test/");
        assert_eq!(runtime_state.pop_prefix(), "");
        runtime_state.set_prefix("test/test/");
        assert_eq!(runtime_state.pop_prefix(), "test/");
    }
}
