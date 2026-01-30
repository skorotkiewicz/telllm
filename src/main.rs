mod llm;
mod logger;
mod session;

use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::llm::LlmClient;
use crate::session::Session;

/// Telnet server for chatting with LLM
#[derive(Parser, Debug)]
#[command(name = "telllm")]
#[command(about = "Telnet server for LLM chat with OpenAI-compatible API")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "2323")]
    port: u16,

    /// LLM API endpoint
    #[arg(short, long, default_value = "http://localhost:8080/v1")]
    endpoint: String,

    /// Model name
    #[arg(short, long, default_value = "default")]
    model: String,

    /// API key (optional)
    #[arg(short = 'k', long, default_value = "")]
    api_key: String,

    /// Custom system prompt
    #[arg(short, long, default_value = "You are a helpful AI assistant. Be concise and friendly.")]
    system_prompt: String,

    /// Logs directory
    #[arg(long, default_value = "logs")]
    logs_dir: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive("telllm=info".parse()?))
        .init();

    let args = Args::parse();

    info!("Starting telllm server on port {}", args.port);
    info!("LLM endpoint: {}", args.endpoint);
    info!("Model: {}", args.model);
    info!("Logs directory: {}", args.logs_dir);

    let llm_client = Arc::new(LlmClient::new(
        args.endpoint.clone(),
        args.model.clone(),
        args.api_key.clone(),
    ));

    let system_prompt = Arc::new(args.system_prompt.clone());
    let logs_dir = Arc::new(args.logs_dir.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = TcpListener::bind(addr).await?;

    info!("Listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New connection from {}", addr);
                
                let llm = Arc::clone(&llm_client);
                let prompt = Arc::clone(&system_prompt);
                let logs = Arc::clone(&logs_dir);
                
                tokio::spawn(async move {
                    let mut session = Session::new(stream, addr, llm, prompt, logs);
                    if let Err(e) = session.run().await {
                        error!("Session error for {}: {}", addr, e);
                    }
                    info!("Connection closed: {}", addr);
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
