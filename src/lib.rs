use aws_sdk_s3::model::{CommonPrefix, Object};

pub mod frontend;

// UI는 RuntimeState를 화면에 그리면 된다
pub struct RuntimeState {
    // 현재 선택된 버켓
    bucket: Option<String>,
    // 현재 조회하고 있는 prefix
    prefix: String,
    // 현재 조회한 prefix 내 존재하는 directories
    common_prefix: Option<Vec<CommonPrefix>>,
    // 현재 조회한 prefix 내 존재하는 left key
    contents: Option<Vec<Object>>,
}

impl RuntimeState {
    pub fn new() -> RuntimeState {
        RuntimeState {
            bucket: Default::default(),
            prefix: Default::default(),
            common_prefix: Default::default(),
            contents: Default::default(),
        }
    }

    pub fn directories(&self) -> &Option<Vec<CommonPrefix>> {
        &self.common_prefix
    }

    pub fn keys(&self) -> &Option<Vec<Object>> {
        &self.contents
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn set_prefix(&mut self, prefix: &str) {
        self.prefix = prefix.into()
    }

    pub fn pop(&mut self) {
        if !self.prefix.is_empty() {
            assert!(self.prefix.ends_with("/"));
            let mut split = self.prefix.split("/").collect::<Vec<_>>();
            split.pop(); // last member is empty string
            split.pop(); // delete last component
            if split.len() > 0 {
                self.prefix = split.join("/") + "/";
            } else {
                self.prefix.clear();
            }
        }
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
        runtime_state.pop();
        assert_eq!(runtime_state.prefix, "");
        runtime_state.set_prefix("test/test/");
        runtime_state.pop();
        assert_eq!(runtime_state.prefix, "test/");
    }
}
