# Contributing

Welcome! This guide will help you get started contributing to this project.

## Quick Start

1. **Prerequisites**: Install [Git](https://git-scm.com/), [Docker](https://docker.com), and [Homebrew](https://brew.sh/) (Mac/Linux)
2. **Setup**: Run `make setup` to install all dependencies
3. **Development**: Create a feature branch: `{issue-number}-{your-username}`
4. **Testing**: Run `make test` before submitting
5. **Submit**: Create a pull request using our template

## Development Workflow

### Branching Strategy

- `main` branch is our primary branch
- Create feature branches: `10-johnsmith` (issue number + username)
- Use `HOTFIX-description` for emergency fixes

### Making Changes

1. Create a feature branch from `main`
2. Make your changes following our [standards and best practices](docs/standards/README.md)
3. Write tests for new features
4. Run `make lint` and `make test`
5. Commit using [conventional commits](https://www.conventionalcommits.org/)

### Pull Requests

- Draft PR early with initial documentation commit
- Use `Resolves #10` to link to issues
- Fill out the PR template completely
- Ensure all CI checks pass

## Project Structure

```
├── README.md              # Project overview
├── CONTRIBUTING.md        # This file
├── documentation/         # Technical docs and ADRs
├── .github/              # GitHub templates and workflows
├── .vscode/              # VS Code settings
└── Makefile              # Development commands
```

## Environment Setup

### Required Tools

- **Git**: Version control
- **Docker**: Container platform
- **Python 3.10+**: Via pyenv recommended
- **Node.js**: Via n package manager recommended
- **Make**: For running development commands

### Installation

```bash
# Install pyenv and Python
brew install pyenv
pyenv install 3.10 && pyenv global 3.10

# Install n and Node.js
brew install n
sudo n install latest && sudo n latest

# Install development tools
brew install pre-commit commitizen
npm install -g pnpm

# Setup project
git clone <repo>
cd <repo>
make setup
```

## Development Commands

```bash
make help          # Show all available commands
make setup         # Setup development environment
make start         # Start development services
make test          # Run all tests
make lint          # Run code linting
make commit        # Interactive commit with validation
make clean         # Clean up generated files
```

## Code Standards Summary

- **Python**: Black formatting, flake8 linting, type hints, pytest for testing
- **TypeScript**: Prettier formatting, ESLint linting, Jest for testing
- **General**: Conventional commits, 80%+ test coverage, meaningful names

For detailed standards, see [Standards and Best Practices](docs/standards/README.md).

## Testing Requirements

- Write unit tests for all new features
- Maintain 80%+ test coverage
- All tests must pass before merging
- Integration tests for API endpoints

## Need Help?

- 📖 [Technical Specification](docs/README.md)
- 🏗️ [Architecture Decisions](docs/adrs/README.md)
- 🔧 [Setup Troubleshooting](docs/setup/TROUBLESHOOTING.md)
- 🐛 [Bug Reports](.github/ISSUE_TEMPLATE/bug-report.yaml)
- 💡 [Feature Requests](.github/ISSUE_TEMPLATE/feature.yaml)

## Resources

- [Standards and Best Practices](docs/standards/README.md)
- [Infrastructure Guide](docs/standards/infrastructure/README.md)
- [Setup Guide](docs/setup/README.md)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)

---

**Note**: This project follows a comprehensive set of standards to ensure quality and consistency. The detailed documentation is available in the `docs/` directory.
