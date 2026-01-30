use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tracing::{info, warn};

use crate::llm::{LlmClient, Message};
use crate::logger::ChatLogger;

const WELCOME_BANNER: &str = r#"
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   ████████╗███████╗██╗     ██╗     ██╗     ███╗   ███╗        ║
║   ╚══██╔══╝██╔════╝██║     ██║     ██║     ████╗ ████║        ║
║      ██║   █████╗  ██║     ██║     ██║     ██╔████╔██║        ║
║      ██║   ██╔══╝  ██║     ██║     ██║     ██║╚██╔╝██║        ║
║      ██║   ███████╗███████╗███████╗███████╗██║ ╚═╝ ██║        ║
║      ╚═╝   ╚══════╝╚══════╝╚══════╝╚══════╝╚═╝     ╚═╝        ║
║                                                               ║
║           Telnet LLM Chat Server                              ║
╚═══════════════════════════════════════════════════════════════╝

Commands:
  /name <your name>  - Set your name
  /clear             - Clear conversation history
  /help              - Show this help
  /quit              - Disconnect

Type your message and press Enter to chat with the AI.
"#;

enum CommandResult {
    Quit,
    Continue,
    Message(String),
}

struct SessionState {
    messages: Vec<Message>,
    user_name: Option<String>,
}

impl SessionState {
    fn new(system_prompt: &str) -> Self {
        Self {
            messages: vec![Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            }],
            user_name: None,
        }
    }

    fn handle_command(&mut self, input: &str, logger: &ChatLogger, addr: &SocketAddr) -> CommandResult {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim());

        match cmd.as_str() {
            "/quit" | "/exit" | "/q" => CommandResult::Quit,
            "/name" => {
                if let Some(name) = arg {
                    self.user_name = Some(name.to_string());
                    if let Err(e) = logger.update_summary("name", name) {
                        return CommandResult::Message(format!("\nError saving name: {}\n", e));
                    }
                    info!("User {} set name to: {}", addr, name);
                    CommandResult::Message(format!("\nName set to: {}\n", name))
                } else {
                    CommandResult::Message("\nUsage: /name <your name>\n".to_string())
                }
            }
            "/clear" => {
                // Keep only system prompt
                self.messages.truncate(1);
                info!("User {} cleared conversation", addr);
                CommandResult::Message("\nConversation cleared.\n".to_string())
            }
            "/help" | "/?" => {
                CommandResult::Message(
                    "\nCommands:\n\
                      /name <your name>  - Set your name\n\
                      /clear             - Clear conversation history\n\
                      /help              - Show this help\n\
                      /quit              - Disconnect\n"
                        .to_string(),
                )
            }
            _ => CommandResult::Message(format!("\nUnknown command: {}\n", cmd)),
        }
    }
}

pub struct Session {
    stream: TcpStream,
    addr: SocketAddr,
    llm: Arc<LlmClient>,
    system_prompt: Arc<String>,
    logs_dir: Arc<String>,
}

impl Session {
    pub fn new(
        stream: TcpStream,
        addr: SocketAddr,
        llm: Arc<LlmClient>,
        system_prompt: Arc<String>,
        logs_dir: Arc<String>,
    ) -> Self {
        Self {
            stream,
            addr,
            llm,
            system_prompt,
            logs_dir,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let logger = ChatLogger::new(&self.logs_dir, self.addr.ip())?;
        logger.log_session_start()?;

        let mut state = SessionState::new(&self.system_prompt);

        // Load existing summary to get user name
        if let Some(summary) = logger.get_summary() {
            for line in summary.lines() {
                if line.to_lowercase().starts_with("name:") {
                    state.user_name = line.splitn(2, ':').nth(1).map(|s| s.trim().to_string());
                }
            }
        }

        let (read_half, write_half) = self.stream.split();
        let mut reader = BufReader::new(read_half);
        let mut writer = BufWriter::new(write_half);

        // Send welcome banner
        writer.write_all(WELCOME_BANNER.as_bytes()).await?;
        
        if let Some(name) = &state.user_name {
            writer
                .write_all(format!("\nWelcome back, {}!\n\n", name).as_bytes())
                .await?;
        }
        
        writer.write_all(b"\nYou: ").await?;
        writer.flush().await?;

        let mut line = String::new();
        
        loop {
            line.clear();
            
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // Connection closed
                    break;
                }
                Ok(_) => {
                    let input = line.trim().to_string();
                    
                    if input.is_empty() {
                        writer.write_all(b"You: ").await?;
                        writer.flush().await?;
                        continue;
                    }

                    // Handle commands
                    if input.starts_with('/') {
                        match state.handle_command(&input, &logger, &self.addr) {
                            CommandResult::Quit => {
                                writer.write_all(b"\nGoodbye!\n").await?;
                                writer.flush().await?;
                                break;
                            }
                            CommandResult::Continue => {
                                writer.write_all(b"\nYou: ").await?;
                                writer.flush().await?;
                                continue;
                            }
                            CommandResult::Message(msg) => {
                                writer.write_all(msg.as_bytes()).await?;
                                writer.write_all(b"\nYou: ").await?;
                                writer.flush().await?;
                                continue;
                            }
                        }
                    }

                    // Log user message
                    let display_name = state.user_name.as_deref().unwrap_or("User");
                    logger.log_message(display_name, &input)?;

                    // Add user message to history
                    state.messages.push(Message {
                        role: "user".to_string(),
                        content: input.clone(),
                    });

                    // Show typing indicator
                    writer.write_all(b"\nAI: (thinking...)\r").await?;
                    writer.flush().await?;

                    // Call LLM
                    match self.llm.chat(&state.messages).await {
                        Ok(response) => {
                            // Clear the thinking indicator and show response
                            writer
                                .write_all(format!("AI: {}\n", response).as_bytes())
                                .await?;

                            // Log and store response
                            logger.log_message("AI", &response)?;
                            state.messages.push(Message {
                                role: "assistant".to_string(),
                                content: response,
                            });
                        }
                        Err(e) => {
                            warn!("LLM error for {}: {}", self.addr, e);
                            writer
                                .write_all(
                                    format!("AI: Sorry, I encountered an error: {}\n", e)
                                        .as_bytes(),
                                )
                                .await?;
                        }
                    }

                    writer.write_all(b"\nYou: ").await?;
                    writer.flush().await?;
                }
                Err(e) => {
                    return Err(e).context("Failed to read from client");
                }
            }
        }

        logger.log_session_end()?;
        Ok(())
    }
}
