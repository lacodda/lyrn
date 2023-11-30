use json_value_merge::Merge;
use serde_json::Value;

pub trait ValueExt {
    fn merge_default(&mut self, value: &Value);
}

impl ValueExt for Value {
    fn merge_default(&mut self, value: &Value) {
        if !value.is_null() {
            self.merge(value.clone());
        }
    }
}
