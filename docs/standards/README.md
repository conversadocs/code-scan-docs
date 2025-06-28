# Code Standards

This document outlines the general coding standards and best practices that apply across all programming languages in this project.

## Overview

Our code standards ensure consistency, maintainability, and quality across the entire codebase. These principles apply regardless of the programming language being used.

## Universal Principles

### Code Quality

- **Readability First**: Code is read more often than it's written
- **Simplicity**: Prefer simple solutions over complex ones
- **Consistency**: Follow established patterns within the codebase
- **Single Responsibility**: Each function/class should have one clear purpose
- **DRY (Don't Repeat Yourself)**: Avoid code duplication
- **YAGNI (You Aren't Gonna Need It)**: Don't implement features until needed

### Naming Conventions

- **Be Descriptive**: Names should clearly indicate purpose
- **Use Full Words**: Avoid abbreviations unless universally understood
- **Be Consistent**: Use the same naming patterns throughout
- **Avoid Mental Mapping**: Names should be self-explanatory

```javascript
// Good
const getUserById = (userId) => { ... }
const isUserActive = (user) => { ... }
const MAX_RETRY_ATTEMPTS = 3

// Bad
const getUsrById = (id) => { ... }
const chkUsr = (u) => { ... }
const MAX_RETRIES = 3 // inconsistent with other constants
```

### Function Design

- **Small Functions**: Functions should be small and focused
- **Pure Functions**: Prefer functions without side effects when possible
- **Clear Parameters**: Function signatures should be self-documenting
- **Return Early**: Use guard clauses to reduce nesting

```python
# Good
def calculate_discount(user, order_total):
    if not user.is_premium:
        return 0

    if order_total < 100:
        return order_total * 0.05

    return order_total * 0.10

# Bad
def calculate_discount(user, order_total):
    discount = 0
    if user.is_premium:
        if order_total >= 100:
            discount = order_total * 0.10
        else:
            discount = order_total * 0.05
    return discount
```

### Error Handling

- **Fail Fast**: Detect and report errors as early as possible
- **Be Specific**: Use specific error types and messages
- **Handle Gracefully**: Provide meaningful error messages to users
- **Log Appropriately**: Log errors at the appropriate level
- **Don't Swallow Errors**: Always handle or propagate errors

### Comments and Documentation

- **Code Should Be Self-Documenting**: Write clear code that doesn't need comments
- **Comment Why, Not What**: Explain the reasoning behind complex logic
- **Keep Comments Up-to-Date**: Maintain comments when code changes
- **Use Standard Documentation Formats**: Follow language-specific documentation standards

```typescript
// Good: Explains why this approach was chosen
// Using exponential backoff to avoid overwhelming the API
// during high traffic periods
const delay = Math.min(1000 * Math.pow(2, retryCount), 30000)

// Bad: Explains what the code does (obvious)
// Multiply 1000 by 2 raised to the power of retryCount
const delay = 1000 * Math.pow(2, retryCount)
```

## Testing Standards

### Test Coverage

- **Minimum 80% code coverage** across all projects
- **100% coverage for critical business logic**
- **Test edge cases and error conditions**
- **Write tests before fixing bugs** (regression tests)

### Test Structure

- **Arrange, Act, Assert (AAA)** pattern for test organization
- **Descriptive test names** that explain the scenario
- **Independent tests** that don't rely on each other
- **Fast tests** that run quickly in CI/CD pipelines

### Test Types

1. **Unit Tests** (70%): Test individual functions/methods
2. **Integration Tests** (20%): Test component interactions
3. **End-to-End Tests** (10%): Test complete user workflows

## Version Control Standards

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting changes
- `refactor`: Code refactoring
- `test`: Adding tests
- `chore`: Maintenance tasks

**Examples:**

```
feat(auth): add JWT token validation
fix(api): handle null user ID in profile endpoint
docs(readme): update installation instructions
refactor(utils): extract common validation logic
```

### Branch Naming

- **Feature branches**: `{issue-number}-{username}` (e.g., `123-johnsmith`)
- **Hotfix branches**: `hotfix-{description}` (e.g., `hotfix-security-patch`)
- **Multiple features**: `{issue-number}-{username}-{number}` (e.g., `123-johnsmith-2`)

### Pull Request Standards

- **Link to issues**: Use `Resolves #123` in PR description
- **Small, focused changes**: Keep PRs manageable in size
- **Complete feature**: PRs should include tests and documentation
- **Self-review**: Review your own code before requesting review

## Security Standards

### Input Validation

- **Validate all inputs** at application boundaries
- **Sanitize user input** to prevent injection attacks
- **Use parameterized queries** for database operations
- **Validate file uploads** for type and size

### Authentication & Authorization

- **Never store passwords in plain text**
- **Use strong, unique secrets** for each environment
- **Implement proper session management**
- **Follow principle of least privilege**

### Data Protection

- **Encrypt sensitive data** both at rest and in transit
- **Use HTTPS** for all web communications
- **Avoid logging sensitive information**
- **Implement proper data retention policies**

## Performance Standards

### General Guidelines

- **Measure before optimizing**: Use profiling tools
- **Optimize for readability first**, performance second
- **Cache expensive operations** when appropriate
- **Use efficient algorithms and data structures**

### Database Interactions

- **Use connection pooling** for database connections
- **Implement proper indexing** for frequently queried fields
- **Avoid N+1 query problems**
- **Use pagination** for large result sets

### API Design

- **Implement rate limiting** to prevent abuse
- **Use compression** for large responses
- **Implement caching headers** appropriately
- **Design RESTful endpoints** with proper HTTP methods

## Code Review Standards

### What to Look For

- **Correctness**: Does the code do what it's supposed to do?
- **Security**: Are there any security vulnerabilities?
- **Performance**: Are there obvious performance issues?
- **Maintainability**: Is the code easy to understand and modify?
- **Testing**: Are there adequate tests for the changes?

### Review Process

1. **Automated checks first**: Ensure CI/CD passes before human review
2. **Review within 24 hours**: Don't let PRs sit idle
3. **Be constructive**: Provide helpful feedback, not just criticism
4. **Ask questions**: If something is unclear, ask for clarification
5. **Approve when ready**: Don't nitpick minor style issues

## Language-Specific Standards

For detailed language-specific standards, see:

- [JavaScript/TypeScript Standards](javascript.md)
- [Python Standards](python.md)
- [Rust Standards](rust.md)

## Infrastructure Standards

For infrastructure and deployment standards, see:

- [Infrastructure Standards](infrastructure/README.md)
- [Docker Standards](infrastructure/docker.md)
- [Terraform Standards](infrastructure/terraform.md)
- [CloudFormation Standards](infrastructure/cloudFormation.md)
- [Kubernetes Standards](infrastructure/kubernetes.md)

## Tools and Automation

### Required Tools

- **Linters**: Language-specific linting tools
- **Formatters**: Automatic code formatting
- **Type Checkers**: Static type analysis where applicable
- **Security Scanners**: Automated vulnerability detection
- **Test Runners**: Automated testing frameworks

### CI/CD Requirements

- **Automated testing**: All tests must pass before merge
- **Code coverage**: Must meet minimum thresholds
- **Security scanning**: Automated security checks
- **Code formatting**: Automatic formatting validation
- **Dependency scanning**: Check for vulnerable dependencies

## Enforcement

### Pre-commit Hooks

All repositories must have pre-commit hooks that check:

- Code formatting
- Linting rules
- Basic security scans
- Test execution
- Commit message format

### CI/CD Pipeline

All code changes must pass through automated checks:

- Compilation/syntax validation
- Unit test execution
- Integration test execution
- Security vulnerability scanning
- Code coverage reporting
- Static code analysis

### Manual Review

Human code review is required for:

- All production code changes
- Security-sensitive modifications
- Architecture changes
- Public API modifications

---

Remember: These standards exist to help us write better code together. When in doubt, prioritize code clarity and team consistency over personal preferences.
