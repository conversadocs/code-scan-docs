"""Base classes for CSD plugins."""

from .input import (
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
from .output import (
    BaseOutputPlugin,
    OutputPluginInput,
    OutputPluginResult,
    calculate_file_metrics,
    extract_dependencies,
)

__all__ = [
    "BaseAnalyzer",
    "PluginInput",
    "PluginOutput",
    "BaseOutputPlugin",
    "OutputPluginInput",
    "OutputPluginResult",
    "calculate_file_metrics",
    "extract_dependencies",
    "calculate_complexity",
    "detect_import_type",
    "CodeElement",
    "Import",
    "Relationship",
    "ExternalDependency",
]
