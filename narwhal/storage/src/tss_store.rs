use types::message_out::keygen_result::KeygenResultData;

#[derive(Clone, Default)]
pub struct TssStore {
    key_data: Option<KeygenResultData>,
}

impl TssStore {
    pub fn new() -> Self {
        Self { key_data: None }
    }
    pub fn set_key(&mut self, key_data: KeygenResultData) {
        self.key_data = Some(key_data);
    }
}
