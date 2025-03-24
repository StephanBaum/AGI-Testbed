# AGI Testbed

A comprehensive testing framework for Artificial General Intelligence components based on the specifications outlined in the MVP document.

## Overview

This testbed verifies key features of the proposed AGI architecture:
- **Variable Time-Scaling**: Dynamically adjusts processing rates based on task complexity/urgency
- **Continuous Memory Management**: Shifts capacity between short-term and long-term storage
- **Inference-Based Reasoning**: Maintains structural knowledge representations and logical inference
- **Persistent Operational States**: Demonstrates autonomous operation in multiple modes
- **Operational Observability, Security, & Resilience**: Ensures maintainability, reliability, and safety

## System Architecture

The system consists of a Core Orchestrator that coordinates multiple specialized instances:

1. **Core Orchestrator**:
   - Manages inter-instance communication
   - Ensures operational mode persistence
   - Enforces safety & alignment rules
   - Distributes workloads and allocates resources

2. **Component Instances**:
   - **Variable Time-Scaling Instance**: Dynamically adjusts processing rates
   - **Continuous Memory Management Instance**: Shifts capacity between STM and LTM
   - **Inference-Based Reasoning Core Instance**: Maintains knowledge representations
   - **Persistent Operational State Instance**: Demonstrates autonomous operation

3. **Web Interface**:
   - System dashboard for monitoring all components
   - Component-specific control panels
   - Testbed execution and monitoring
   - Visualization of metrics and results

## Installation

### Prerequisites

1. **Install Rust**:
   - Windows: Download and run [rustup-init.exe](https://win.rustup.rs/)
   - macOS/Linux: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   - Follow the on-screen instructions
   - Restart your terminal after installation

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

## Usage

### Starting the System

1. Run the application:
   ```
   cargo run
   ```

2. Open your web browser and navigate to:
   ```
   http://localhost:8080
   ```

3. The main dashboard will show the status of all components.

### Component Management

Each component can be managed through its dedicated page:

- **Time Scaling**: Adjust processing modes and observe scaling behavior
- **Memory Management**: Configure memory allocation and observe item transfers
- **Reasoning**: Build knowledge graphs and run inference operations
- **Operational State**: Monitor autonomous operation and goal achievement

### Running Testbeds

The system includes four main testbeds:

1. **Testbed 1: Complexity-Based Time Scaling**
   - Validates dynamic CPU/GPU time allocation
   - Tests different complexity levels and processing modes
   - Measures completion time and response quality

2. **Testbed 2: Dynamic Memory Allocation**
   - Measures STM vs. LTM performance during high-load conditions
   - Tests different memory allocation ratios
   - Evaluates transfer and consolidation operations

3. **Testbed 3: Structural Reasoning Validation**
   - Validates knowledge integration & inference correctness
   - Tests rule-based reasoning and inference patterns
   - Measures inference accuracy and performance

4. **Testbed 4: Continuous Autonomous Operation**
   - Assesses system's ability to run with changing goals
   - Tests recovery from interruptions
   - Evaluates mode-switching and resource allocation

## Project Structure

- `/src/core/` - Core orchestrator and component interfaces
- `/src/instances/` - Implementations of the four main component instances
- `/src/web/` - Web interface and API
  - `/src/web/handlers/` - API endpoint handlers
  - `/src/web/templates/` - HTML templates
  - `/src/web/static/` - CSS, JavaScript, and other static assets
- `/docs/` - Documentation and project logs

## Development Log

See [DEVLOG.md](./docs/DEVLOG.md) for a detailed record of development progress and notes.

## Future Improvements

- Add more sophisticated visualizations for metrics data
- Implement more advanced test scenarios for each component
- Enhance the inference engine with more complex reasoning patterns
- Add authentication and user management for multi-user scenarios
- Create exportable reports for test results
- Add integration with external monitoring tools

## License

This project is licensed under the MIT License - see the LICENSE file for details.