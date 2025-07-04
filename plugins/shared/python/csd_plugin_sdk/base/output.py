#!/usr/bin/env python3
"""
Base class for CSD output plugins.
All output plugins should inherit from this class.
"""
import io
import typing
import json
import sys
import time
import hashlib
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Dict, List, Optional, Any, Tuple
from dataclasses import dataclass, asdict


@dataclass
class OutputPluginInput:
    """Input data for output plugins."""

    matrix_path: str
    project_root: str
    output_dir: str
    cache_dir: str
    plugin_config: Optional[Dict[str, Any]] = None
    format_options: Dict[str, Any] = None

    def __post_init__(self):
        """Initialize default values after dataclass initialization."""
        if self.format_options is None:
            self.format_options = {}


@dataclass
class GeneratedOutput:
    """Individual output file or result generated by an output plugin."""

    output_path: str
    content_type: str  # "markdown", "html", "json", "pdf", etc.
    size_bytes: int
    checksum: str
    metadata: Dict[str, Any] = None

    def __post_init__(self):
        """Initialize default metadata if not provided."""
        if self.metadata is None:
            self.metadata = {}


@dataclass
class OutputPluginResult:
    """Result from an output plugin."""

    plugin_name: str
    plugin_version: str
    output_type: str  # "documentation", "quality_report", "security_scan", etc.
    outputs: List[GeneratedOutput]
    processing_time_ms: int = 0
    metadata: Dict[str, Any] = None

    def __post_init__(self):
        """Initialize default metadata if not provided."""
        if self.metadata is None:
            self.metadata = {}


class BaseOutputPlugin(ABC):
    """Base class for all CSD output plugins."""

    def __init__(self):
        """Initialize the base output plugin."""
        self.name = self.__class__.__name__
        self.version = "1.0.0"
        self.plugin_type = "output"
        self.supported_output_types = []
        self.supported_formats = []

    @abstractmethod
    def can_generate(self, output_type: str, format: str) -> Tuple[bool, float]:
        """
        Check if this plugin can generate the given output type and format.

        Args:
            output_type: Type of output (e.g., "documentation", "quality_report")
            format: Output format (e.g., "markdown", "html", "pdf")

        Returns:
            Tuple containing can_generate (bool) and confidence (float).
        """
        pass

    @abstractmethod
    def generate(self, input_data: OutputPluginInput) -> OutputPluginResult:
        """
        Generate output based on the project matrix.

        Args:
            input_data: Output plugin input containing matrix path and configuration

        Returns:
            OutputPluginResult with generated outputs
        """
        pass

    def get_info(self) -> Dict[str, Any]:
        """Return plugin information."""
        return {
            "name": self.name,
            "version": self.version,
            "plugin_type": self.plugin_type,
            "supported_extensions": [],
            "supported_filenames": [],
            "supported_output_types": self.supported_output_types,
            "supported_formats": self.supported_formats,
        }

    def _generate_output_filename(
        self, base_name: str, format: str, output_dir: str
    ) -> str:
        """Generate a unique output filename."""
        timestamp = int(time.time())
        clean_name = base_name.replace(" ", "_").replace("/", "_").replace("\\", "_")

        ext = self._get_extension_for_format(format)

        filename = f"{clean_name}_{timestamp}{ext}"
        return str(Path(output_dir) / filename)

    def _get_extension_for_format(self, format: str) -> str:
        """Get file extension for a given format."""
        extensions = {
            "markdown": ".md",
            "html": ".html",
            "pdf": ".pdf",
            "json": ".json",
            "yaml": ".yaml",
            "xml": ".xml",
            "csv": ".csv",
            "txt": ".txt",
            "rst": ".rst",
            "latex": ".tex",
        }
        return extensions.get(format.lower(), ".txt")

    def _calculate_file_checksum(self, file_path: str) -> str:
        """Calculate SHA256 checksum of a file."""
        try:
            with open(file_path, "rb") as f:
                content = f.read()
                return hashlib.sha256(content).hexdigest()
        except Exception:
            return "error"

    def _create_generated_output(
        self,
        output_path: str,
        content_type: str,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> GeneratedOutput:
        """Create a GeneratedOutput object with calculated size and checksum."""
        path_obj = Path(output_path)

        size_bytes = 0
        checksum = "pending"

        if path_obj.exists():
            size_bytes = path_obj.stat().st_size
            checksum = self._calculate_file_checksum(output_path)

        return GeneratedOutput(
            output_path=output_path,
            content_type=content_type,
            size_bytes=size_bytes,
            checksum=checksum,
            metadata=metadata or {},
        )

    def _load_matrix_from_file(self, matrix_path: str) -> Dict[str, Any]:
        """Load and parse the project matrix from JSON file."""
        try:
            with open(matrix_path, "r", encoding="utf-8") as f:
                matrix_data: Dict[str, Any] = json.load(f)
                return matrix_data
        except Exception as e:
            raise RuntimeError(f"Failed to load matrix from {matrix_path}: {e}")

    def _ensure_output_directory(self, output_dir: str) -> None:
        """Ensure the output directory exists."""
        Path(output_dir).mkdir(parents=True, exist_ok=True)

    def run(self):
        """Main entry point for plugin execution."""
        try:
            typing.cast(io.TextIOWrapper, sys.stdout).reconfigure(line_buffering=True)
            typing.cast(io.TextIOWrapper, sys.stderr).reconfigure(line_buffering=True)

            try:
                input_data = sys.stdin.read().strip()
            except Exception as e:
                self._send_error(f"Failed to read input: {e}")
                input_lines = []
                try:
                    for line in sys.stdin:
                        input_lines.append(line.strip())
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

            try:
                message = json.loads(input_data)
            except json.JSONDecodeError as e:
                self._send_error(f"Invalid JSON: {e}")
                return

            if message.get("type") == "can_generate":
                self._handle_can_generate(message)
            elif message.get("type") == "generate":
                self._handle_generate(message)
            elif message.get("type") == "get_info":
                self._handle_get_info()
            else:
                self._send_error(f"Unknown message type: {message.get('type')}")

        except Exception as e:
            import traceback

            error_details = traceback.format_exc()
            self._send_error(f"Plugin error: {e}", error_details)

    def _handle_can_generate(self, message: Dict[str, Any]):
        """Handle can_generate request."""
        try:
            output_type = message["output_type"]
            format = message["format"]

            can_generate, confidence = self.can_generate(output_type, format)

            response = {
                "status": "can_generate",
                "can_generate": can_generate,
                "confidence": confidence,
            }

            self._send_response(response)

        except Exception as e:
            self._send_error(f"Error in can_generate: {e}")

    def _handle_generate(self, message: Dict[str, Any]):
        """Handle generate request."""
        try:
            input_dict = message["input"]
            input_data = OutputPluginInput(**input_dict)

            start_time = time.time()
            result = self.generate(input_data)
            end_time = time.time()

            result.processing_time_ms = int((end_time - start_time) * 1000)
            result.plugin_version = self.version

            response = {
                "status": "output_success",
                "result": asdict(result),
            }

            self._send_response(response)

        except Exception as e:
            self._send_error(f"Error in generate: {e}")

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


def calculate_file_metrics(matrix_data: Dict[str, Any]) -> Dict[str, Any]:
    """
    Calculate basic file-level metrics from matrix data.
    Utility function for output plugins.
    """
    files = matrix_data.get("files", {})

    total_files = len(files)
    total_size = sum(file_info.get("size_bytes", 0) for file_info in files.values())

    by_plugin: Dict[str, int] = {}
    total_elements = 0
    total_complexity = 0

    for file_info in files.values():
        plugin = file_info.get("plugin", "unknown")
        by_plugin[plugin] = by_plugin.get(plugin, 0) + 1

        elements = file_info.get("elements", [])
        total_elements += len(elements)

        for element in elements:
            complexity = element.get("complexity_score", 0)
            if complexity:
                total_complexity += complexity

    return {
        "total_files": total_files,
        "total_size_bytes": total_size,
        "total_size_mb": total_size / (1024 * 1024),
        "files_by_plugin": by_plugin,
        "total_elements": total_elements,
        "total_complexity": total_complexity,
        "average_complexity": total_complexity / max(total_elements, 1),
    }


def extract_dependencies(
    matrix_data: Dict[str, Any],
) -> Dict[str, List[Dict[str, Any]]]:
    """
    Extract and organize external dependencies by ecosystem.
    Utility function for output plugins.
    """
    dependencies = matrix_data.get("external_dependencies", [])

    by_ecosystem: Dict[str, List[Dict[str, Any]]] = {}
    for dep in dependencies:
        ecosystem = dep.get("ecosystem", "unknown")
        if ecosystem not in by_ecosystem:
            by_ecosystem[ecosystem] = []
        by_ecosystem[ecosystem].append(dep)

    return by_ecosystem


def build_dependency_graph(matrix_data: Dict[str, Any]) -> Dict[str, List[str]]:
    """
    Build a simple dependency graph from relationships.
    Utility function for output plugins.
    """
    relationships = matrix_data.get("relationships", [])

    graph: Dict[str, List[str]] = {}
    for rel in relationships:
        from_file = rel.get("from_file", "")
        to_file = rel.get("to_file", "")

        if from_file and to_file:
            if from_file not in graph:
                graph[from_file] = []
            graph[from_file].append(to_file)

    return graph
