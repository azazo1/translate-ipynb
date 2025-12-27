use rig::{
    agent::Agent,
    client::CompletionClient,
    completion::{Prompt, PromptError},
    http_client,
    providers::openai::{Client, CompletionModel},
};

pub struct TranslateAgent {
    agent: Agent<CompletionModel>,
}

impl TranslateAgent {
    pub fn new(
        api_key: &str,
        base_url: &str,
        model: &str,
        language: &str,
    ) -> Result<Self, http_client::Error> {
        let agent = Client::<reqwest::Client>::builder()
            .base_url(base_url)
            .api_key(api_key)
            .build()?
            .completions_api()
            .completion_model(model)
            .into_agent_builder()
            .preamble(&format!(r#"I want you to act as an {language} translator. I will speak to you in any language and you will detect the language, translate it and answer in {language}. Keep the meaning and structure same, preserving whitespaces and newlines. Do not write explanations. Do not translate codes."#))
            .build();
        Ok(TranslateAgent { agent })
    }

    pub async fn translate(
        &self,
        content: &str,
    ) -> Result<String, PromptError> {
        self.agent.prompt(content).await
    }
}
