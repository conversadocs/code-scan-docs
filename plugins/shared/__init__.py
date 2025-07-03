"""
Shared utilities for CSD plugins.
"""

# Input plugin base classes and utilities
from .base_analyzer import (
    BaseAnalyzer,
    CodeElement,
    Import,
    Relationship,
    ExternalDependency,
    PluginInput,
    PluginOutput,
    calculate_complexity,
    detect_import_type,
)

# Output plugin base classes and utilities
from .base_output import (
    BaseOutputPlugin,
    OutputPluginInput,
    GeneratedOutput,
    OutputPluginResult,
    calculate_file_metrics,
    extract_dependencies,
    build_dependency_graph,
)

__all__ = [
    # Input plugin classes
    "BaseAnalyzer",
    "CodeElement",
    "Import",
    "Relationship",
    "ExternalDependency",
    "PluginInput",
    "PluginOutput",
    "calculate_complexity",
    "detect_import_type",
    # Output plugin classes
    "BaseOutputPlugin",
    "OutputPluginInput",
    "GeneratedOutput",
    "OutputPluginResult",
    "calculate_file_metrics",
    "extract_dependencies",
    "build_dependency_graph",
]
