use super::model::{
    CodeLanguage, InferenceStats, PlaygroundModel, RoutingTier, MODEL_CATALOG, STREAMED_RESPONSE,
};

pub const DEFAULT_SYSTEM_PROMPT: &str = "You are an expert full-stack engineer and distributed systems architect. Provide precise, actionable technical answers.";
pub const DEFAULT_USER_PROMPT: &str =
    "Explain how hardware attestation guarantees secure multi-tenant LLM token generation.";
pub const STREAM_CHUNK_SIZE: usize = 12;

#[derive(Clone, Debug, PartialEq)]
pub struct PlaygroundState {
    pub selected_model_index: usize,
    pub tier: RoutingTier,
    pub system_prompt: String,
    pub user_prompt: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub code_language: CodeLanguage,
    pub copied: bool,
    pub running: bool,
    pub stream_id: u64,
    pub output: String,
    pub stats: Option<InferenceStats>,
}

impl Default for PlaygroundState {
    fn default() -> Self {
        Self {
            selected_model_index: 0,
            tier: RoutingTier::Standard,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            user_prompt: DEFAULT_USER_PROMPT.to_string(),
            temperature: 0.7,
            max_tokens: 2_048,
            code_language: CodeLanguage::Curl,
            copied: false,
            running: false,
            stream_id: 0,
            output: String::new(),
            stats: None,
        }
    }
}

impl PlaygroundState {
    pub fn selected_model(&self) -> PlaygroundModel {
        MODEL_CATALOG
            .get(self.selected_model_index)
            .copied()
            .unwrap_or(MODEL_CATALOG[0])
    }

    pub fn set_model_id(&mut self, id: &str) {
        if let Some(index) = MODEL_CATALOG.iter().position(|model| model.id == id) {
            self.selected_model_index = index;
        }
    }

    pub fn clear_output(&mut self) {
        self.running = false;
        self.stream_id = self.stream_id.wrapping_add(1);
        self.output.clear();
        self.stats = None;
    }

    pub fn begin_inference(&mut self) {
        self.stream_id = self.stream_id.wrapping_add(1);
        self.running = true;
        self.output.clear();
        self.stats = None;
    }

    pub fn append_stream_chunk(&mut self, length: usize) {
        let mut boundary = length.min(STREAMED_RESPONSE.len());
        while boundary > 0 && !STREAMED_RESPONSE.is_char_boundary(boundary) {
            boundary -= 1;
        }
        self.output = STREAMED_RESPONSE[..boundary].to_string();
    }

    pub fn complete_inference(&mut self, total_ms: u32) {
        self.output = STREAMED_RESPONSE.to_string();
        self.running = false;
        self.stats = Some(InferenceStats {
            ttft_ms: self.tier.ttft_ms(),
            total_ms,
            prompt_tokens: 42,
            completion_tokens: 184,
            cost_usd: estimate_cost(self.selected_model()),
        });
    }
}

pub fn estimate_cost(model: PlaygroundModel) -> f64 {
    ((42.0 * model.input_price_per_million + 184.0 * model.output_price_per_million) / 1_000_000.0)
        .round_to_six_places()
}

trait RoundToSixPlaces {
    fn round_to_six_places(self) -> Self;
}

impl RoundToSixPlaces for f64 {
    fn round_to_six_places(self) -> Self {
        (self * 1_000_000.0).round() / 1_000_000.0
    }
}
