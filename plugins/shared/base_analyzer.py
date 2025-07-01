#!/usr/bin/env python3
"""
Base class for CSD plugins.
All language plugins should inherit from this class.
"""

import json
import sys
import time
import hashlib
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Dict, List, Optional, Any, Union
from dataclasses import dataclass, asdict


@dataclass
class CodeElement:
    element_type: str  # 'function', 'class', 'method', 'variable', etc.
    name: str
    signature: Optional[str] = None
    line_start: int = 0
    line_end: int = 0
    summary: Optional[str] = None
    complexity_score: Optional[int] = None
    calls: List[str] = None
    metadata: Dict[str, Any] = None

    def __post_init__(self):
        if self.calls is None:
            self.calls = []
        if self.metadata is None:
            self.metadata = {}


@dataclass
class Import:
    module: str
    items: List[str] = None
    alias: Optional[str] = None
    line_number: int = 0
    import_type: str = "standard"  # 'standard', 'third_party', 'local', 'relative'

    def __post_init__(self):
        if self.items is None:
            self.items = []


@dataclass
class Relationship:
    from_file: str
    to_file: str
    relationship_type: str
    details: str
    line_number: Optional[int] = None
    strength: float = 1.0


@dataclass
class ExternalDependency:
    name: str
    version: Optional[str] = None
    ecosystem: str = "unknown"
    dependency_type: str = "runtime"  # 'runtime', 'development', 'build', 'optional'
    source_file: str = ""


@dataclass
class PluginOutput:
    file_path: str
    file_hash: str
    elements: List[CodeElement]
    imports: List[Import]
    exports: List[str]
    relationships: List[Relationship]
    external_dependencies: List[ExternalDependency]
    file_summary: Optional[str] = None
    processing_time_ms: int = 0
    plugin_version: str = "1.0.0"


@dataclass
class PluginInput:
    file_path: str
    relative_path: str
    content: str
    project_root: str
    cache_dir: str  # New field for cache directory
    plugin_config: Optional[Dict[str, Any]] = None


class BaseAnalyzer(ABC):
    """Base class for all CSD plugins."""

    def __init__(self):
        self.name = self.__class__.__name__
        self.version = "1.0.0"
        self.supported_extensions = []
        self.supported_filenames = []

    @abstractmethod
    def can_analyze(self, file_path: str, content_preview: str) -> tuple[bool, float]:
        """
        Check if this plugin can analyze the given file.

        Args:
            file_path: Path to the file
            content_preview: First ~500 characters of the file

        Returns:
            (can_analyze: bool, confidence: float)
        """
        pass

    @abstractmethod
    def analyze(self, input_data: PluginInput) -> PluginOutput:
        """
        Analyze the given file and return structured data.

        Args:
            input_data: Plugin input containing file content and metadata

        Returns:
            PluginOutput with analysis results
        """
        pass

    def get_info(self) -> Dict[str, Any]:
        """Return plugin information."""
        return {
            "name": self.name,
            "version": self.version,
            "supported_extensions": self.supported_extensions,
            "supported_filenames": self.supported_filenames,
        }

    def _generate_cache_filename(self, input_data: PluginInput) -> str:
        """Generate a unique cache filename for this analysis."""
        # Create a hash from file path and content to ensure uniqueness
        content_hash = hashlib.md5(
            (input_data.file_path + input_data.content).encode()
        ).hexdigest()[:16]

        # Clean the relative path for filename use
        clean_path = input_data.relative_path.replace("/", "_").replace("\\", "_")
        clean_path = clean_path.replace(".", "_")

        return f"{self.name}_{clean_path}_{content_hash}.json"

    def _write_to_cache(
        self, result: PluginOutput, cache_filename: str, cache_dir: str
    ) -> str:
        """Write analysis result to cache file and return the filename."""
        cache_path = Path(cache_dir) / cache_filename

        # Ensure cache directory exists
        cache_path.parent.mkdir(parents=True, exist_ok=True)

        # Write the result
        with open(cache_path, "w", encoding="utf-8") as f:
            json.dump(asdict(result), f, indent=2, ensure_ascii=False)

        return cache_filename

    def run(self):
        """Main entry point for plugin execution."""
        try:
            # Configure stdout to be line buffered
            sys.stdout.reconfigure(line_buffering=True)
            sys.stderr.reconfigure(line_buffering=True)

            # Read all input from stdin - try different approaches
            try:
                # Method 1: Read all at once
                input_data = sys.stdin.read().strip()
            except Exception as e:
                # Method 2: Read line by line until we get valid JSON
                print(f"Failed to read all input: {e}", file=sys.stderr)
                input_lines = []
                try:
                    for line in sys.stdin:
                        input_lines.append(line.strip())
                        # Try to parse as JSON to see if we have a complete message
                        try:
                            full_input = "".join(input_lines)
                            json.loads(full_input)
                            input_data = full_input
                            break
                        except json.JSONDecodeError:
                            continue
                    else:
                        input_data = "".join(input_lines)
                except Exception as e2:
                    self._send_error(f"Failed to read input: {e2}")
                    return

            if not input_data:
                self._send_error("No input received")
                return

            # Parse the message
            try:
                message = json.loads(input_data)
            except json.JSONDecodeError as e:
                self._send_error(f"Invalid JSON: {e}")
                return

            # Handle the message
            if message.get("type") == "can_analyze":
                self._handle_can_analyze(message)
            elif message.get("type") == "analyze":
                self._handle_analyze(message)
            elif message.get("type") == "get_info":
                self._handle_get_info()
            else:
                self._send_error(f"Unknown message type: {message.get('type')}")

        except Exception as e:
            import traceback

            error_details = traceback.format_exc()
            self._send_error(f"Plugin error: {e}", error_details)

    def _handle_can_analyze(self, message: Dict[str, Any]):
        """Handle can_analyze request."""
        try:
            file_path = message["file_path"]
            content_preview = message["content_preview"]

            can_analyze, confidence = self.can_analyze(file_path, content_preview)

            response = {
                "status": "can_analyze",
                "can_analyze": can_analyze,
                "confidence": confidence,
            }

            self._send_response(response)

        except Exception as e:
            self._send_error(f"Error in can_analyze: {e}")

    def _handle_analyze(self, message: Dict[str, Any]):
        """Handle analyze request - now writes to cache file."""
        try:
            input_dict = message["input"]
            input_data = PluginInput(**input_dict)

            start_time = time.time()
            result = self.analyze(input_data)
            end_time = time.time()

            # Update timing
            result.processing_time_ms = int((end_time - start_time) * 1000)
            result.plugin_version = self.version

            # Generate cache filename and write to cache
            cache_filename = self._generate_cache_filename(input_data)
            actual_filename = self._write_to_cache(
                result, cache_filename, input_data.cache_dir
            )

            # Send back just the cache filename, not the full data
            response = {
                "status": "success",
                "cache_file": actual_filename,
                "processing_time_ms": result.processing_time_ms,
            }

            self._send_response(response)

        except Exception as e:
            self._send_error(f"Error in analyze: {e}")

    def _handle_get_info(self):
        """Handle get_info request."""
        try:
            info = self.get_info()
            response = {"status": "info", **info}

            self._send_response(response)

        except Exception as e:
            self._send_error(f"Error in get_info: {e}")

    def _send_response(self, response: Dict[str, Any]):
        """Send a response to stdout."""
        json_response = json.dumps(response)
        print(json_response)
        sys.stdout.flush()

    def _send_error(self, message: str, details: Optional[str] = None):
        """Send an error response."""
        response = {"status": "error", "message": message, "details": details}
        self._send_response(response)


def calculate_complexity(code: str, element_start: int, element_end: int) -> int:
    """
    Calculate a simple complexity score for a code element.
    This is a basic implementation - can be enhanced per language.
    """
    lines = code.split("\n")[element_start - 1 : element_end]
    code_block = "\n".join(lines)

    # Simple complexity heuristics
    complexity = 1  # Base complexity

    # Control flow statements add complexity
    complexity_keywords = [
        "if",
        "elif",
        "else",
        "for",
        "while",
        "try",
        "except",
        "match",
        "case",
    ]
    for keyword in complexity_keywords:
        complexity += code_block.count(f" {keyword} ") + code_block.count(
            f"\t{keyword} "
        )

    # Nested functions add complexity
    complexity += code_block.count("def ") - 1  # Subtract 1 for the function itself

    return max(1, complexity)


def detect_import_type(module_name: str, project_root: str, file_path: str) -> str:
    """
    Detect the type of import based on the module name and project structure.
    """
    if module_name.startswith("."):
        return "relative"

    # Check if it's a local module (exists in project)
    project_path = Path(project_root)
    file_dir = Path(file_path).parent

    # Try to find the module in the project
    potential_paths = [
        project_path / f"{module_name.replace('.', '/')}.py",
        project_path / f"{module_name.replace('.', '/')}/__init__.py",
        file_dir / f"{module_name.replace('.', '/')}.py",
        file_dir / f"{module_name.replace('.', '/')}/__init__.py",
    ]

    for path in potential_paths:
        if path.exists():
            return "local"

    # Check if it's a standard library module
    standard_modules = {
        "os",
        "sys",
        "json",
        "time",
        "datetime",
        "pathlib",
        "collections",
        "itertools",
        "functools",
        "typing",
        "dataclasses",
        "abc",
        "re",
        "math",
        "random",
        "urllib",
        "http",
        "socket",
        "threading",
        "asyncio",
        "logging",
        "argparse",
        "configparser",
        "csv",
        "xml",
        "sqlite3",
    }

    root_module = module_name.split(".")[0]
    if root_module in standard_modules:
        return "standard"

    return "third_party"
