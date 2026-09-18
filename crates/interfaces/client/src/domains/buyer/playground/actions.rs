use super::{
    model::CodeLanguage,
    state::{PlaygroundState, DEFAULT_SYSTEM_PROMPT, DEFAULT_USER_PROMPT},
};

fn escape_code_string(value: &str) -> String {
    value.replace('"', "\\\"")
}

pub fn code_snippet(state: &PlaygroundState) -> String {
    let model = state.selected_model();
    let system_prompt = escape_code_string(&state.system_prompt);
    let user_prompt = escape_code_string(&state.user_prompt);
    let tier = state.tier.api_name();
    let temperature = format!("{:.1}", state.temperature);

    match state.code_language {
        CodeLanguage::Curl => format!(
            "curl https://api.burncloud.io/v1/chat/completions \\\n  -H \"Content-Type: application/json\" \\\n  -H \"Authorization: Bearer $BURNCLOUD_API_KEY\" \\\n  -H \"X-BurnCloud-Tier: {tier}\" \\\n  -d '{{\n    \"model\": \"{}\",\n    \"messages\": [\n      {{\"role\": \"system\", \"content\": \"{system_prompt}\"}},\n      {{\"role\": \"user\", \"content\": \"{user_prompt}\"}}\n    ],\n    \"temperature\": {temperature},\n    \"max_tokens\": {}\n  }}'",
            model.id, state.max_tokens
        ),
        CodeLanguage::Python => format!(
            "from openai import OpenAI\n\n# BurnCloud endpoint is 100% drop-in compatible with standard OpenAI SDKs\nclient = OpenAI(\n    api_key=\"bc_live_your_key_here\",\n    base_url=\"https://api.burncloud.io/v1\",\n    default_headers={{\"X-BurnCloud-Tier\": \"{tier}\"}}\n)\n\nresponse = client.chat.completions.create(\n    model=\"{}\",\n    messages=[\n        {{\"role\": \"system\", \"content\": \"{system_prompt}\"}},\n        {{\"role\": \"user\", \"content\": \"{user_prompt}\"}}\n    ],\n    temperature={temperature},\n    max_tokens={}\n)\n\nprint(response.choices[0].message.content)",
            model.id, state.max_tokens
        ),
        CodeLanguage::Node => format!(
            "import OpenAI from \"openai\";\n\nconst openai = new OpenAI({{\n  apiKey: process.env.BURNCLOUD_API_KEY,\n  baseURL: \"https://api.burncloud.io/v1\",\n  defaultHeaders: {{\n    \"X-BurnCloud-Tier\": \"{tier}\"\n  }}\n}});\n\nasync function main() {{\n  const completion = await openai.chat.completions.create({{\n    model: \"{}\",\n    messages: [\n      {{ role: \"system\", content: \"{system_prompt}\" }},\n      {{ role: \"user\", content: \"{user_prompt}\" }}\n    ],\n    temperature: {temperature},\n    max_tokens: {}\n  }});\n\n  console.log(completion.choices[0].message.content);\n}}\n\nmain();",
            model.id, state.max_tokens
        ),
    }
}

pub fn reset_prompts(state: &mut PlaygroundState) {
    state.system_prompt = DEFAULT_SYSTEM_PROMPT.to_string();
    state.user_prompt = DEFAULT_USER_PROMPT.to_string();
}
