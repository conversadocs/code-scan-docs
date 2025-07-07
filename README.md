# code-scan-docs

`code-scan-docs` is an open-source tool that leverages AI to analyze your codebase, generating comprehensive documentation and conducting automated code reviews. By scanning your project's files and mapping their relationships, it provides valuable insights into dependencies and potential code issues, streamlining development and documentation workflows.

## Overview

`code-scan-docs` simplifies the process of understanding and documenting large codebases through intelligent automation. Built with a high-performance Rust core and extensible Python plugins, it recursively scans your project directories to identify each file, determine its role within the project, and map out relationships between files and functions. This process generates a detailed matrix of information, including summaries, dependency graphs, and usage relationships.

Armed with this in-depth analysis, `code-scan-docs` can perform automated code reviews, identifying bad coding practices and potential security vulnerabilities. It also facilitates the creation of comprehensive documentation by generating content or populating existing templates, ensuring that your documentation stays up-to-date with minimal manual effort. Whether you're aiming to improve code quality, enhance security, or streamline documentation, `code-scan-docs` integrates seamlessly into your development workflow to support these goals.

## Installation

### Prerequisites

- **Rust**: Version 1.70 or higher
- **Python**: Version 3.8 or higher (for plugins)
- **Git**: For repository scanning features

### Install from Source

1. Clone the repository:

```bash
git clone <repository-url>
cd csd
```

2. Build the application:

```bash
cargo build --release
```

3. Install globally (optional):

```bash
cargo install --path .
```

### Install Python Plugin Dependencies

The application uses Python-based plugins for code analysis. Install the required dependencies:

```bash
cd plugins/shared/python
pip install -r requirements.txt
# or for development
pip install -e .
```

### Verify Installation

Check that CSD is properly installed:

```bash
csd --version
```

## Usage

### Quick Start

1. **Initialize a project scan**:

```bash
csd init
```

This will scan your current directory and create a `.csd_cache/matrix.json` file with the analysis results.

2. **Generate documentation**:

```bash
csd docs --format markdown
```

3. **View available plugins**:

```bash
csd plugins
```

### Commands

### `csd init`

Scan and analyze your project to build the code matrix.

```bash
# Scan current directory
csd init

# Scan specific directory
csd init /path/to/project

# Export results in different formats
csd init --output json --output-file analysis.json
csd init --output yaml --output-file analysis.yaml
```

**Options:**

- `--output <format>`: Export format (json, yaml)
- `--output-file <path>`: Output file path
- `--no-llm`: Disable LLM features
- `--include-tests`: Include test files in analysis

#### `csd docs`

Generate documentation from your project matrix.

```bash
# Generate markdown documentation
csd docs --format markdown

# Specify output directory
csd docs --format markdown --output-dir ./docs

# Use custom matrix file
csd docs --matrix custom-matrix.json --format html
```

#### `csd plugins`

List and manage available plugins.

```bash
# List all plugins
csd plugins

# Show detailed plugin information
csd plugins --detailed
```

#### `csd config`

Initialize or manage configuration.

```bash
# Create default configuration file
csd config

# Force overwrite existing config
csd config --force
```

### Configuration

CSD uses a `.csdrc.yaml` configuration file in your project root:

```yaml
# Example configuration
project:
  name: 'My Project'
  description: 'Project description'

input_plugins:
  rust_analyzer:
    enabled: true
    path: 'plugins/input/code/rust_analyzer.py'
  python_analyzer:
    enabled: true
    path: 'plugins/input/code/python_analyzer.py'

output_plugins:
  markdown_docs:
    enabled: true
    path: 'plugins/output/docs/markdown_docs.py'
    config:
      llm:
        enabled: true
        provider: 'ollama'
        model: 'llama2'

cache:
  directory: '.csd_cache'
  enabled: true

scanning:
  ignore_patterns:
    - 'node_modules'
    - 'target'
    - '.git'
    - '*.log'
```

### Plugin System

CSD uses a plugin architecture for extensible code analysis:

#### Input Plugins

- **Rust Analyzer**: Analyzes Rust code structure, functions, modules
- **Python Analyzer**: Analyzes Python code, imports, classes, functions

#### Output Plugins

- **Markdown Documentation**: Generates comprehensive markdown docs with optional LLM enhancement

#### LLM Integration

Enable LLM-powered documentation enhancement:

```yaml
output_plugins:
  markdown_docs:
    config:
      llm:
        enabled: true
        provider: 'ollama' # or "openai"
        model: 'llama2'
        base_url: 'http://localhost:11434' # for Ollama
```

### Examples

#### Basic Project Analysis

```bash
# Scan and generate docs in one go
csd init && csd docs --format markdown --output-dir ./documentation
```

#### Advanced Usage

```bash
# Scan with custom config
csd init --config custom-config.yaml

# Generate docs with LLM enhancement
csd docs --format markdown --output-dir ./docs

# Analyze specific directory
csd init /path/to/source/code --output json --output-file analysis.json
```

### File Structure

After running CSD, your project will have:

```
your-project/
├── .csdrc.yaml          # Configuration file
├── .csd_cache/          # Cache directory
│   └── matrix.json      # Project analysis matrix
└── docs/                # Generated documentation (if specified)
    └── README.md
```

## Troubleshooting

### Common Issues

1. **Python plugin errors**: Ensure Python dependencies are installed

```bash
cd plugins/shared/python && pip install -e .
```

2. **Plugin not found**: Check plugin paths in `.csdrc.yaml`

3. **Permission errors**: Ensure CSD has read access to your project files

### Debug Mode

Run with verbose logging:

```bash
RUST_LOG=debug csd init
```

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://www.jasonanton.com"><img src="https://avatars.githubusercontent.com/u/6391564?v=4?s=100" width="100px;" alt="Jason Anton"/><br /><sub><b>Jason Anton</b></sub></a><br /><a href="#content-stoopidJSON" title="Content">🖋</a> <a href="https://github.com/conversadocs/code-scan-docs/commits?author=stoopidJSON" title="Code">💻</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!
