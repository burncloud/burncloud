use super::{
    actions::{
        create_api_key, generate_secret, revoke_api_key, CreateKeyError, CreateKeyInput,
        DEFAULT_MONTHLY_SPEND_CAP, DEFAULT_RATE_LIMIT_RPM,
    },
    model::BuyerApiKey,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ApiKeysState {
    pub keys: Vec<BuyerApiKey>,
    pub create_modal_open: bool,
    pub result_modal_open: bool,
    pub new_key_name: String,
    pub new_key_rate_limit: u32,
    pub new_key_cap: f64,
    pub created_secret: Option<String>,
    pub copied: bool,
    pub validation_error: Option<CreateKeyError>,
    next_local_key_id: u64,
}

impl Default for ApiKeysState {
    fn default() -> Self {
        Self {
            keys: BuyerApiKey::mock_keys(),
            create_modal_open: false,
            result_modal_open: false,
            new_key_name: String::new(),
            new_key_rate_limit: DEFAULT_RATE_LIMIT_RPM,
            new_key_cap: DEFAULT_MONTHLY_SPEND_CAP,
            created_secret: None,
            copied: false,
            validation_error: None,
            next_local_key_id: 4,
        }
    }
}

impl ApiKeysState {
    pub fn active_count(&self) -> usize {
        self.keys
            .iter()
            .filter(|key| key.status.is_active())
            .count()
    }

    pub fn open_create_modal(&mut self) {
        self.create_modal_open = true;
        self.validation_error = None;
    }

    pub fn close_create_modal(&mut self) {
        self.create_modal_open = false;
        self.validation_error = None;
    }

    pub fn create_key(&mut self) -> Result<(), CreateKeyError> {
        self.create_key_with_secret(generate_secret())
    }

    pub fn create_key_with_secret(&mut self, secret: String) -> Result<(), CreateKeyError> {
        let input = CreateKeyInput {
            name: self.new_key_name.clone(),
            rate_limit_rpm: self.new_key_rate_limit,
            monthly_spend_cap: self.new_key_cap,
        };
        let id = format!("key-local-{}", self.next_local_key_id);
        let result = match create_api_key(id, input, secret) {
            Ok(result) => result,
            Err(error) => {
                self.validation_error = Some(error);
                return Err(error);
            }
        };

        self.keys.insert(0, result.key);
        self.created_secret = Some(result.secret);
        self.create_modal_open = false;
        self.result_modal_open = true;
        self.new_key_name.clear();
        self.copied = false;
        self.validation_error = None;
        self.next_local_key_id += 1;
        Ok(())
    }

    pub fn close_result_modal(&mut self) {
        self.result_modal_open = false;
        self.created_secret = None;
        self.copied = false;
    }

    pub fn mark_copied(&mut self) {
        if self.created_secret.is_some() {
            self.copied = true;
        }
    }

    pub fn clear_copied(&mut self) {
        self.copied = false;
    }

    pub fn revoke_key(&mut self, id: &str) -> bool {
        revoke_api_key(&mut self.keys, id)
    }
}
