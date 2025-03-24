# AGI Testbed

A testing framework for Artificial General Intelligence components based on the specifications outlined in the MVP document.

## Overview

This testbed verifies key features of the proposed AGI architecture:
- Variable Time-Scaling
- Continuous Memory Management
- Inference-Based Reasoning
- Persistent Operational States
- Operational Observability, Security, & Resilience

## Installation

### Prerequisites

1. **Install Rust**:
   - Windows: Download and run [rustup-init.exe](https://win.rustup.rs/)
   - Follow the on-screen instructions to complete the installation
   - Restart your terminal/command prompt after installation

2. **Verify Installation**:
   ```
   rustc --version
   cargo --version
   ```

### Setup Project

```bash
# Clone the repository (if applicable)
# git clone <repository-url>

# Navigate to the project directory
cd agi_testbed

# Build the project
cargo build

# Run the project
cargo run
```

## Project Structure

- `/src` - Rust source code for the AGI testbed components
- `/ui` - Web interface for visualization and management
- `/docs` - Documentation and project logs

## Components

1. **Core Orchestrator** - Manages inter-instance communication and ensures operational persistence
2. **Variable Time-Scaling Instance** - Dynamically adjusts processing rates
3. **Continuous Memory Management Instance** - Shifts capacity between short-term and long-term storage
4. **Inference-Based Reasoning Core Instance** - Maintains structural knowledge representations
5. **Persistent Operational State Instance** - Demonstrates autonomous operation in multiple modes

## Usage

Detailed usage instructions will be provided as the project develops.

## Development Log

See [DEVLOG.md](./docs/DEVLOG.md) for development progress and notes.