pub mod frontend;

pub struct RuntimeState {
    prefix: String,
}

impl RuntimeState {
    pub fn new() -> RuntimeState {
        RuntimeState {
            prefix: String::default(),
        }
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
