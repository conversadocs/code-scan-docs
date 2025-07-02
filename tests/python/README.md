# CSD Python Plugin Tests

This directory contains comprehensive tests for the CSD Python plugins. These tests serve both as verification for the built-in plugins and as examples/templates for external plugin developers.

## Project Structure

```
csd/                          # Project root
├── pyproject.toml           # Python project config (includes pytest settings)
├── Cargo.toml               # Rust configuration
├── src/                     # Rust source code
├── plugins/                 # Python plugins
│   ├── python_analyzer.py
│   ├── rust_analyzer.py
│   └── shared/
└── tests/
    ├── python/              # Python plugin tests
    │   ├── requirements.txt
    │   ├── conftest.py
    │   ├── test_base_analyzer.py
    │   ├── README.md (this file)
    │   └── run_tests.py
    └── integration/         # Future mixed Rust/Python tests
```

## Quick Start

### 1. Install Test Dependencies

```bash
# From the project root
pip install -r tests/python/requirements.txt
```

### 2. Run All Python Tests

```bash
# From the project root
pytest tests/python/ -v

# Or using the test runner script
python tests/python/run_tests.py
```

### 3. Run Specific Test Files

```bash
# Test just the base analyzer
pytest tests/python/test_base_analyzer.py -v

# Using the test runner
python tests/python/run_tests.py base

# Test with coverage
pytest tests/python/ --cov=plugins --cov-report=term-missing
python tests/python/run_tests.py --coverage
```

## Test Organization

```
tests/python/
├── requirements.txt          # Test dependencies
├── conftest.py              # Shared fixtures and utilities
├── test_base_analyzer.py    # Base analyzer tests ✅
├── test_python_analyzer.py  # Python plugin tests (coming next)
├── test_rust_analyzer.py    # Rust plugin tests (coming next)
├── README.md                # This documentation
├── run_tests.py             # Convenient test runner
└── fixtures/                # Test data files (coming next)
```

**Note**: Pytest configuration is included in the `pyproject.toml` file at the project root using the modern `[tool.pytest.ini_options]` section.

## Configuration

The pytest configuration is located in the project root `pyproject.toml` file under the `[tool.pytest.ini_options]` section. This modern approach manages all Python testing for the project:

```toml
[tool.pytest.ini_options]
testpaths = ["tests/python"]
python_files = "test_*.py"
addopts = [
    "-v",
    "--tb=short",
    "--strict-markers",
    "--cov=plugins",
    "--cov-report=term-missing"
]
markers = [
    "unit: Unit tests for individual components",
    "integration: Integration tests for plugin communication",
    "slow: Tests that take longer to run"
]
```

Benefits of the pyproject.toml approach:

- **Modern standard**: Follows PEP 518 for Python project configuration
- **Single file**: All Python project settings in one place
- **Tool integration**: Better support in modern IDEs and tools

### Running from Different Locations

```bash
# From project root (recommended)
pytest tests/python/

# From tests/python/ directory
cd tests/python/
pytest .

# Using the test runner from anywhere
python tests/python/run_tests.py
```

### Data Structures Tests (`TestDataStructures`)

- Test creation and validation of core plugin data structures
- `CodeElement`, `Import`, `Relationship`, `ExternalDependency`
- Both minimal and complete object creation scenarios

### Utility Functions Tests (`TestUtilityFunctions`)

- Test helper functions like `calculate_complexity()` and `detect_import_type()`
- Edge cases and different input scenarios

### Base Analyzer Tests (`TestBaseAnalyzer`)

- Abstract class behavior
- Cache filename generation
- File I/O operations

### Plugin Communication Tests (`TestPluginCommunication`)

- JSON protocol handling
- Error scenarios
- Message validation

## For External Plugin Developers

These tests demonstrate:

1. **Required Data Structures**: How to properly create and populate the plugin output structures
2. **Utility Functions**: Helper functions available for complexity analysis and import detection
3. **Communication Protocol**: How plugins communicate with the main CSD application
4. **Testing Patterns**: How to structure tests for your own plugins

### Example: Testing Your Own Plugin

```python
# In your plugin tests
from base_analyzer import PluginInput, PluginOutput
from conftest import assert_valid_json_communication

def test_my_plugin_analysis():
    # Use the same patterns as the built-in tests
    input_data = PluginInput(...)
    result = my_plugin.analyze(input_data)

    assert isinstance(result, PluginOutput)
    assert len(result.elements) > 0
    # ... more assertions
```

## Test Fixtures

The `conftest.py` file provides useful fixtures:

- `temp_project_dir`: Temporary directory for test projects
- `sample_plugin_input`: Ready-to-use PluginInput object
- `sample_code_elements`: Example code elements
- `complete_plugin_output`: Fully populated PluginOutput
- Helper functions for JSON validation and file creation

## Coverage Requirements

Tests maintain 80% minimum coverage across the plugin codebase. Run with coverage:

```bash
pytest tests/python/ --cov=plugins --cov-report=html
# Open htmlcov/index.html to view detailed coverage report
```

## Running Tests in Development

For active development, use pytest's watch mode:

```bash
# Install pytest-watch
pip install pytest-watch

# Watch for changes and re-run tests
ptw tests/python/ --runner "pytest --tb=short"
```

## Integration with Pre-commit

These tests are integrated with the pre-commit hooks to ensure code quality before commits. The pytest configuration in `pyproject.toml` allows the pre-commit system to run Python tests alongside Rust tests as part of the quality gate.

### Pre-commit Hooks for Python:

The `.pre-commit-config.yaml` includes these Python checks:

```yaml
# Python code formatting (auto-fix)
- id: python-black
  name: Python format (black auto-fix)
  entry: black
  args: [--line-length=88]

# Python linting
- id: python-flake8
  name: Python linting (flake8)
  entry: flake8
  args: [--max-line-length=88, --extend-ignore=E203, W503]

# Python type checking
- id: python-mypy
  name: Python type checking (mypy)
  entry: mypy
  args: [--ignore-missing-imports, --no-strict-optional]

# Python plugin tests
- id: python-tests
  name: Python plugin tests
  entry: pytest tests/python/ -v --tb=short
  always_run: true
```

### Setup Development Environment:

```bash
# Run the setup script (from project root)
./setup_python_dev.sh

# Or manually install dependencies
pip install -r tests/python/requirements.txt
pre-commit install

# Run pre-commit manually
pre-commit run --all-files
```

### Development Workflow:

#### **Local Development (with quality checks):**

```bash
# Run tests with coverage (local insight)
pytest tests/python/ --cov=plugins --cov-report=html

# Format code (pre-commit will also do this)
black plugins/ tests/python/

# Check linting (pre-commit will also do this)
flake8 plugins/ tests/python/

# Type checking (optional, for better code quality)
mypy plugins/
```

#### **Pre-commit Workflow:**

1. **Code Formatting**: Black automatically formats Python code
2. **Linting**: Flake8 checks for style and potential issues
3. **Type Checking**: MyPy validates type hints
4. **Tests**: Full Python plugin test suite runs
5. **Rust Checks**: Existing Rust quality gates also run

If any check fails, the commit is blocked until issues are resolved.

#### **CI/CD Workflow:**

- **Pre-commit**: Quality checks (formatting, linting, type checking, tests)
- **GitHub Actions**: Test execution only (clean, fast pipeline)
- **Philosophy**: Fix issues before they reach the repo, CI just validates tests pass
