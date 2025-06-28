# ADR-0002: Flexible LLM Integration via RESTful APIs

## Status

Accepted

## Context

The `code-scan-docs` project relies on Large Language Models (LLMs) to perform code analysis, generate documentation, and identify potential issues within codebases. Initially, integration with specific LLM services like OpenAI's API and Ollama has been implemented. However, the landscape of LLM providers is rapidly evolving, and users may have preferences or requirements to use different LLM services, including self-hosted or locally run models.

Key considerations include:

- **User Flexibility**: Allow users to choose their preferred LLM service based on cost, performance, privacy, or compliance requirements.
- **Scalability**: Support a variety of LLMs to cater to different use cases and codebase sizes.
- **Ease of Integration**: Simplify the process of adding support for new LLMs in the future.
- **Consistency**: Provide a uniform interface for LLM interactions within the application.
- **Technical Constraints**: Ensure that the integration layer handles varying endpoints, authentication methods, and response formats.

## Decision

We have decided to:

- **Implement a Flexible LLM Integration Layer** that allows `code-scan-docs` to interact with any RESTful LLM service.
- **Abstract LLM Interactions** through a standardized interface within the application.
- **Provide Support for OpenAI's API and Ollama** out of the box, serving as initial implementations.
- **Allow Configuration** for end-users to specify their chosen LLM service, including endpoint URLs, authentication credentials, and request parameters.
- **Design for Extensibility**, enabling easy addition of new LLM integrations without significant changes to the core application.

### Rationale

- **User Choice**: Empowering users to select their LLM service accommodates a wider range of needs, such as cost considerations, data privacy, and compliance.
- **Future-Proofing**: As new LLM services emerge, the application can adapt quickly without core architectural changes.
- **Local Deployment**: Supporting local LLMs like Ollama enables offline operation and addresses concerns about sending code to external services.
- **Consistent Experience**: A standardized interface ensures that the application's features work seamlessly regardless of the underlying LLM.
- **Community Contribution**: Simplifies the process for contributors to add support for additional LLMs.

## Consequences

### Positive

- **Flexibility**: Users can integrate any RESTful LLM service that suits their needs.
- **Privacy and Compliance**: Supports local or on-premises LLMs, aiding organizations with strict data policies.
- **Extensibility**: Facilitates the addition of new LLM services with minimal effort.
- **User Adoption**: Broad compatibility may attract a larger user base.

### Negative

- **Complexity**: Handling different LLM APIs increases the complexity of the integration layer.
- **Inconsistencies**: Variations in API formats, authentication methods, and response structures may require additional handling.
- **Maintenance Overhead**: Supporting multiple LLMs may increase the maintenance burden.
- **Limited Features**: Some LLMs may not support all required features, leading to inconsistent capabilities across services.

### Neutral

- **Performance Variability**: Different LLMs may have varying performance characteristics, which is the user's responsibility to manage.

## Implementation

- **Define an Abstract LLM Interface**:
  - Create a standardized interface or set of traits that all LLM integrations must implement.
  - This interface will handle request construction, response parsing, error handling, and authentication mechanisms.
- **Modular LLM Connectors**:
  - Implement connectors for OpenAI's API and Ollama as separate modules adhering to the abstract interface.
  - Each connector handles service-specific details internally.
- **Configuration System**:
  - Extend the application's configuration to allow users to specify:
    - LLM service provider.
    - Endpoint URLs.
    - API keys or authentication tokens.
    - Additional parameters such as model names or versions.
- **Error Handling and Fallbacks**:
  - Implement robust error handling to manage failures or unavailability of the chosen LLM service.
  - Consider fallback mechanisms where appropriate.
- **Documentation**:
  - Provide clear documentation on how to configure and use different LLM services.
  - Include examples and guidance for adding new LLM connectors.

## Alternatives Considered

- **Single LLM Integration**:
  - Only support one LLM service (e.g., OpenAI's API).
  - Rejected due to lack of flexibility and potential issues with vendor lock-in.
- **Plugin-Based LLM Integrations**:
  - Implement LLM integrations as plugins similar to language analysis plugins.
  - Rejected because LLM interactions are core to the application's functionality and benefit from tighter integration.
- **Direct Integration without Abstraction**:
  - Hard-code support for specific LLMs without a standardized interface.
  - Rejected due to scalability issues and increased code duplication.

## References

- [OpenAI API Documentation](https://platform.openai.com/docs/overview)
- [Ollama Documentation](https://ollama.readthedocs.io/en/quickstart/)
- [HTTP Client Libraries in Rust](https://docs.rs/reqwest/latest/reqwest/)
- [Trait Objects in Rust for Abstraction](https://doc.rust-lang.org/book/ch17-02-trait-objects.html)
- [Configuration Management in Rust](https://docs.rs/config/latest/config/)

---

**Date**: 2025-06-28
**Author(s)**: Jason Anton
**Reviewers**: [To be determined]
