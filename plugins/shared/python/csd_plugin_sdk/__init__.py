"""CSD Plugin SDK - Framework for developing CSD plugins."""

from ._version import __version__

from .base.input import (
    BaseAnalyzer,
    PluginInput,
    PluginOutput,
    CodeElement,
    Import,
    Relationship,
    ExternalDependency,
    calculate_complexity,
    detect_import_type,
)
from .base.output import (
    BaseOutputPlugin,
    OutputPluginInput,
    OutputPluginResult,
    calculate_file_metrics,
    extract_dependencies,
)
from .utils.llm import LLMClient, LLMConfig, SectionProcessor
from .utils.llm_providers import (
    LLMProvider,
    LLMResponse,
    OllamaProvider,
    OpenAIProvider,
    get_provider_class,
    register_provider,
)

__all__ = [
    "__version__",
    "BaseAnalyzer",
    "PluginInput",
    "PluginOutput",
    "BaseOutputPlugin",
    "OutputPluginInput",
    "OutputPluginResult",
    "LLMClient",
    "LLMConfig",
    "SectionProcessor",
    "calculate_file_metrics",
    "extract_dependencies",
    "calculate_complexity",
    "detect_import_type",
    "CodeElement",
    "Import",
    "Relationship",
    "ExternalDependency",
    "LLMProvider",
    "LLMResponse",
    "OllamaProvider",
    "OpenAIProvider",
    "get_provider_class",
    "register_provider",
]
