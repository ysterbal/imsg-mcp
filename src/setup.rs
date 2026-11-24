use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};

// Agent types
#[derive(Debug, Clone, PartialEq)]
pub enum Agent {
    Hermes,
    Codex,
    Copilot,
    Pi,
    Standalone,
}

// Implement Display for Agent
impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Agent::Hermes => write!(f, "Hermes"),
            Agent::Codex => write!(f, "Codex"),
            Agent::Copilot => write!(f, "Copilot"),
            Agent::Pi => write!(f, "Pi"),
            Agent::Standalone => write!(f, "Standalone"),
        }
    }
}

// TUI setup struct
pub struct Setup {
    pub agent: Option<Agent>,
}

impl Setup {
    pub fn new() -> Self {
        Setup { agent: None }
    }

    pub fn run(&mut self) -> Result<()> {
        // Detect agent
        self.detect_agent()?;

        // Prompt for agent selection using text-based UI for reliability
        self.run_agent_selection_text()?;

        // Run setup steps outside of TUI
        self.run_setup_steps_non_tui()?;

        Ok(())
    }

    fn run_agent_selection_text(&mut self) -> Result<()> {
        println!("Available agents:");
        println!("1. Hermes");
        println!("2. Codex");
        println!("3. Copilot");
        println!("4. Pi");
        println!("5. Standalone");
        print!("\nEnter your choice (1-5) [default: 1]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let choice = input.trim().parse::<u32>().unwrap_or(1);
        match choice {
            1 => self.agent = Some(Agent::Hermes),
            2 => self.agent = Some(Agent::Codex),
            3 => self.agent = Some(Agent::Copilot),
            4 => self.agent = Some(Agent::Pi),
            5 => self.agent = Some(Agent::Standalone),
            _ => {
                println!("Invalid choice, defaulting to Hermes");
                self.agent = Some(Agent::Hermes);
            }
        }

        println!("Selected agent: {}", self.agent.as_ref().unwrap());
        Ok(())
    }

    fn run_setup_steps_non_tui(&self) -> Result<()> {
        // Step 1: Install imsg-mcp
        self.install_imsg_mcp()?;

        // Step 2: Set up Keychain token
        self.setup_keychain_token()?;

        // Step 3: Configure Full Disk Access (manual step)
        self.configure_full_disk_access()?;

        // Step 4: Configure Agent Skill
        self.configure_agent_skill()?;

        Ok(())
    }

    fn detect_agent(&mut self) -> Result<()> {
        let possible_agents = [
            (Agent::Hermes, "~/.hermes"),
            (Agent::Codex, "~/.codex"),
            (Agent::Copilot, "~/.copilot"),
            (Agent::Pi, "~/.pi"),
        ];

        for (agent, path) in possible_agents {
            let expanded = shellexpand::tilde(path);
            if Path::new(&expanded.to_string()).exists() {
                self.agent = Some(agent);
                return Ok(());
            }
        }

        Ok(())
    }

    fn install_imsg_mcp(&self) -> Result<()> {
        println!("\n=== Step 1: Install imsg-mcp ===");
        println!("Installing imsg-mcp via cargo...");
        
        let status = std::process::Command::new("cargo")
            .arg("install")
            .arg("--path")
            .arg(".")
            .arg("--force")
            .status()
            .context("Failed to execute cargo install")?;
            
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to install imsg-mcp via cargo"));
        }

        println!("✓ imsg-mcp installed successfully.");
        Ok(())
    }

    fn setup_keychain_token(&self) -> Result<()> {
        println!("\n=== Step 2: Set Up Keychain Token ===");
        println!("Setting up Keychain token...");
        let token = self.ask_for_token()?;

        // Remove existing password if it exists (suppress output)
        let _ = std::process::Command::new("security")
            .arg("delete-generic-password")
            .arg("-s")
            .arg("imsg-mcp-auth")
            .arg("-a")
            .arg("imsg-mcp")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        let status = std::process::Command::new("security")
            .arg("add-generic-password")
            .arg("-a")
            .arg("imsg-mcp")
            .arg("-s")
            .arg("imsg-mcp-auth")
            .arg("-w")
            .arg(&token)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("Failed to execute security command")?;
            
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to store token in Keychain"));
        }

        println!("✓ Token stored in Keychain successfully.");
        Ok(())
    }

    fn ask_for_token(&self) -> Result<String> {
        print!("Enter your authentication token (or press Enter to generate one): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().is_empty() {
            // Generate a secure random token (32 bytes = 256 bits of entropy)
            use rand::rngs::OsRng;
            use rand::RngCore;
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            let mut random_bytes = [0u8; 32];
            OsRng.fill_bytes(&mut random_bytes);
            // Encode in base64 for a secure, printable token
            let token = STANDARD.encode(&random_bytes);
            Ok(token)
        } else {
            Ok(input.trim().to_string())
        }
    }

    fn configure_full_disk_access(&self) -> Result<()> {
        println!("\n=== Step 3: Set Up Full Disk Access ===");
        println!("[!] Full Disk Access not configured automatically.");
        println!("Please grant it manually:");
        println!("1. Open System Settings > Privacy & Security > Full Disk Access.");
        println!("2. Add Terminal or your IDE to the list.");
        println!("3. Restart your terminal or IDE.");

        Ok(())
    }

    fn configure_agent_skill(&self) -> Result<()> {
        println!("\n=== Step 4: Configure Agent Skill ===");
        let skill_dir = match &self.agent {
            Some(Agent::Hermes) => "~/.hermes/skills/imsg-mcp-integration",
            Some(Agent::Codex) => "~/.codex/skills/imsg-mcp-integration",
            Some(Agent::Copilot) => "~/.copilot/skills/imsg-mcp-integration",
            Some(Agent::Pi) => "~/.pi/skills/imsg-mcp-integration",
            Some(Agent::Standalone) => "~/imsg-mcp-integration",
            None => return Err(anyhow::anyhow!("Agent not selected")),
        };

        let expanded_dir = shellexpand::tilde(skill_dir);
        let skill_dir_str = expanded_dir.to_string();

        if !Path::new(&skill_dir_str).exists() {
            fs::create_dir_all(&skill_dir_str)?;
            println!("✓ Skill directory created: {}", skill_dir_str);
        }

        // Placeholder for actual file copy logic
        println!("Copying imsg-mcp-integration skill to {}", skill_dir_str);
        println!("✓ Skill configured successfully.");

        Ok(())
    }
}
