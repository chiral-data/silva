mod constants;
mod envs;
mod utils;

mod data_model;
mod ui;
mod entry;
pub use entry::run;

mod config;

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test() {
        use ollama_rs::generation::completion::request::GenerationRequest;
        // use ollama_rs::generation::chat::{request::ChatMessageRequest, ChatMessageResponseStream};
        use tokio::io::{self, AsyncWriteExt};
        use tokio_stream::StreamExt;

        use ollama_rs::Ollama;
        let ollama = Ollama::new("http://100.98.250.114".to_string(), 11434);

        let model = "deepseek-r1:1.5b".to_string();
        let prompt = "Why is the sky blue?".to_string();

        let mut stream = ollama.generate_stream(GenerationRequest::new(model, prompt)).await.unwrap();

        let mut stdout = io::stdout();
        while let Some(res) = stream.next().await {
            let responses = res.unwrap();
            for resp in responses {
                stdout.write_all(resp.response.as_bytes()).await.unwrap();
                stdout.flush().await.unwrap();
            }
        }

    }

}
