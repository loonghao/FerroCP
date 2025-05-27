# Contributing to FerroCP

Thank you for your interest in contributing to FerroCP! Here are some guidelines to help you get started.

## Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/loonghao/FerroCP.git
   cd FerroCP
   ```

2. Install development dependencies with uv:
   ```bash
   uv sync --group all
   ```

3. Build the Rust extension:
   ```bash
   uv run maturin develop --release
   ```

## Code Style

This project uses:
- [Ruff](https://github.com/charliermarsh/ruff) for linting and formatting
- [isort](https://pycqa.github.io/isort/) for import sorting
- [mypy](https://mypy.readthedocs.io/) for type checking

You can run the linters with:
```bash
uv run nox -s lint
```

And fix formatting issues with:
```bash
uv run nox -s lint_fix
```

## Testing

Write tests for all new features and bug fixes. Run the test suite with:
```bash
uv run nox -s test
```

## Pull Request Process

1. Fork the repository and create your branch from `main`.
2. Make your changes and add tests if applicable.
3. Ensure all tests pass and code style checks pass.
4. Update documentation if needed.
5. Submit a pull request.

## Commit Messages

This project follows [Conventional Commits](https://www.conventionalcommits.org/) for commit messages:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types include:
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

## License

By contributing to this project, you agree that your contributions will be licensed under the project's [Apache-2.0 License](LICENSE).
