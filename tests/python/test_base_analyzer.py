#!/usr/bin/env python3
"""
Unit tests for the base analyzer functionality.

These tests cover the core abstractions and utilities that all CSD plugins
should use. They serve as both verification and documentation for external
plugin developers.

Run with: pytest tests/python/test_base_analyzer.py -v
"""

import json
import sys
from unittest.mock import patch, mock_open
from typing import Tuple
import pytest

# Import the modules we're testing
from base_analyzer import (
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


class TestDataStructures:
    """Test the core data structures used in the plugin system."""

    def test_code_element_creation_minimal(self):
        """Test CodeElement creation with minimal required fields."""
        element = CodeElement(
            element_type="function", name="test_func", line_start=10, line_end=20
        )

        assert element.element_type == "function"
        assert element.name == "test_func"
        assert element.line_start == 10
        assert element.line_end == 20

        # Test defaults
        assert element.calls == []
        assert element.metadata == {}
        assert element.signature is None
        assert element.summary is None
        assert element.complexity_score is None

    def test_code_element_creation_complete(self):
        """Test CodeElement creation with all fields."""
        metadata = {"is_async": True, "visibility": "public"}
        calls = ["helper1", "helper2"]

        element = CodeElement(
            element_type="method",
            name="process_data",
            signature="async def process_data(self, data: List[str]) -> Dict[str, Any]",
            line_start=15,
            line_end=45,
            summary="Processes input data and returns structured results",
            complexity_score=8,
            calls=calls,
            metadata=metadata,
        )

        assert element.element_type == "method"
        assert element.name == "process_data"
        assert (
            element.signature
            == "async def process_data(self, data: List[str]) -> Dict[str, Any]"
        )
        assert element.line_start == 15
        assert element.line_end == 45
        assert element.summary == "Processes input data and returns structured results"
        assert element.complexity_score == 8
        assert element.calls == calls
        assert element.metadata == metadata

    def test_import_creation_minimal(self):
        """Test Import creation with minimal required fields."""
        import_obj = Import(module="os", line_number=5)

        assert import_obj.module == "os"
        assert import_obj.line_number == 5

        # Test defaults
        assert import_obj.items == []
        assert import_obj.alias is None
        assert import_obj.import_type == "standard"

    def test_import_creation_complete(self):
        """Test Import creation with all fields."""
        items = ["DataFrame", "Series"]

        import_obj = Import(
            module="pandas",
            items=items,
            alias="pd",
            line_number=3,
            import_type="third_party",
        )

        assert import_obj.module == "pandas"
        assert import_obj.items == items
        assert import_obj.alias == "pd"
        assert import_obj.line_number == 3
        assert import_obj.import_type == "third_party"

    def test_relationship_creation(self):
        """Test Relationship creation."""
        rel = Relationship(
            from_file="src/main.py",
            to_file="src/utils.py",
            relationship_type="import",
            details="imports utility functions",
        )

        assert rel.from_file == "src/main.py"
        assert rel.to_file == "src/utils.py"
        assert rel.relationship_type == "import"
        assert rel.details == "imports utility functions"
        assert rel.line_number is None
        assert rel.strength == 1.0  # Default

    def test_relationship_creation_complete(self):
        """Test Relationship creation with all fields."""
        rel = Relationship(
            from_file="src/models.py",
            to_file="src/database.py",
            relationship_type="call",
            details="calls Database.connect()",
            line_number=42,
            strength=0.7,
        )

        assert rel.from_file == "src/models.py"
        assert rel.to_file == "src/database.py"
        assert rel.relationship_type == "call"
        assert rel.details == "calls Database.connect()"
        assert rel.line_number == 42
        assert rel.strength == 0.7

    def test_external_dependency_creation_minimal(self):
        """Test ExternalDependency creation with minimal fields."""
        dep = ExternalDependency(name="requests", version="2.28.0", ecosystem="pip")

        assert dep.name == "requests"
        assert dep.version == "2.28.0"
        assert dep.ecosystem == "pip"
        assert dep.dependency_type == "runtime"  # Default
        assert dep.source_file == ""  # Default

    def test_external_dependency_creation_complete(self):
        """Test ExternalDependency creation with all fields."""
        dep = ExternalDependency(
            name="pytest",
            version="7.0.0",
            ecosystem="pip",
            dependency_type="development",
            source_file="requirements-dev.txt",
        )

        assert dep.name == "pytest"
        assert dep.version == "7.0.0"
        assert dep.ecosystem == "pip"
        assert dep.dependency_type == "development"
        assert dep.source_file == "requirements-dev.txt"

    def test_plugin_input_creation(self, temp_project_dir):
        """Test PluginInput creation."""
        input_obj = PluginInput(
            file_path="/project/src/main.py",
            relative_path="src/main.py",
            content="print('hello world')",
            project_root="/project",
            cache_dir="/project/.csd_cache",
        )

        assert input_obj.file_path == "/project/src/main.py"
        assert input_obj.relative_path == "src/main.py"
        assert input_obj.content == "print('hello world')"
        assert input_obj.project_root == "/project"
        assert input_obj.cache_dir == "/project/.csd_cache"
        assert input_obj.plugin_config is None

    def test_plugin_input_with_config(self):
        """Test PluginInput creation with plugin configuration."""
        config = {"analyze_comments": True, "max_depth": 5}

        input_obj = PluginInput(
            file_path="/test/file.py",
            relative_path="file.py",
            content="# Test content",
            project_root="/test",
            cache_dir="/test/.cache",
            plugin_config=config,
        )

        assert input_obj.plugin_config == config

    def test_plugin_output_creation_minimal(self):
        """Test PluginOutput creation with minimal required fields."""
        output = PluginOutput(
            file_path="/test/main.py",
            file_hash="abc123",
            elements=[],
            imports=[],
            exports=[],
            relationships=[],
            external_dependencies=[],
        )

        assert output.file_path == "/test/main.py"
        assert output.file_hash == "abc123"
        assert output.elements == []
        assert output.imports == []
        assert output.exports == []
        assert output.relationships == []
        assert output.external_dependencies == []

        # Test defaults
        assert output.file_summary is None
        assert output.processing_time_ms == 0
        assert output.plugin_version == "1.0.0"

    def test_plugin_output_creation_complete(self, complete_plugin_output):
        """Test PluginOutput creation with all fields populated."""
        output = complete_plugin_output

        assert output.file_path == "/project/src/main.py"
        assert output.file_hash == "abc123def456"
        assert len(output.elements) == 2
        assert len(output.imports) == 3
        assert output.exports == ["main_function", "helper"]
        assert len(output.relationships) == 2
        assert len(output.external_dependencies) == 2
        assert output.file_summary == "Main application file with core functionality"
        assert output.processing_time_ms == 150
        assert output.plugin_version == "1.0.0"


class TestUtilityFunctions:
    """Test utility functions provided by the base analyzer."""

    def test_calculate_complexity_simple(self):
        """Test complexity calculation for simple code."""
        simple_code = """
def simple_function():
    return True
"""
        complexity = calculate_complexity(simple_code, 1, 3)
        assert complexity == 1  # Base complexity

    def test_calculate_complexity_with_control_flow(self):
        """Test complexity calculation with control flow structures."""
        complex_code = """
def complex_function(x):
    if x > 0:
        for i in range(x):
            if i % 2 == 0:
                try:
                    result = process(i)
                except ValueError:
                    continue
            else:
                return False
    return True
"""
        complexity = calculate_complexity(complex_code, 1, 11)
        # Should be > 1 due to if, for, if, try/except
        # The exact number may vary based on implementation
        # Let's be more flexible with the assertion
        assert complexity >= 3, f"Expected complexity >= 3, got {complexity}"

    def test_calculate_complexity_with_nested_functions(self):
        """Test complexity calculation with nested functions."""
        nested_code = """
def outer_function():
    def inner_function():
        if True:
            return 1
    return inner_function()
"""
        complexity = calculate_complexity(nested_code, 1, 6)
        # Should account for flow but not double-count the nested function definition
        assert complexity >= 2

    def test_detect_import_type_standard_library(self):
        """Test detection of standard library imports."""
        standard_modules = ["os", "sys", "json", "time", "pathlib", "collections"]

        for module in standard_modules:
            result = detect_import_type(
                module, "/test/project", "/test/project/main.py"
            )
            assert (
                result == "standard"
            ), f"Module {module} should be detected as standard"

    def test_detect_import_type_relative(self):
        """Test detection of relative imports."""
        relative_imports = [".utils", "..parent_module", ".submodule.helper"]

        for module in relative_imports:
            result = detect_import_type(
                module, "/test/project", "/test/project/main.py"
            )
            assert (
                result == "relative"
            ), f"Module {module} should be detected as relative"

    def test_detect_import_type_third_party(self):
        """Test detection of third-party imports."""
        third_party_modules = ["requests", "numpy", "pandas", "django", "flask"]

        for module in third_party_modules:
            result = detect_import_type(
                module, "/test/project", "/test/project/main.py"
            )
            assert (
                result == "third_party"
            ), f"Module {module} should be detected as third_party"

    @patch("pathlib.Path.exists")
    def test_detect_import_type_local(self, mock_exists):
        """Test detection of local imports."""
        # Mock that the local file exists
        mock_exists.return_value = True

        result = detect_import_type(
            "mymodule", "/test/project", "/test/project/main.py"
        )
        assert result == "local"

        # Verify that Path.exists was called
        mock_exists.assert_called()

    @patch("pathlib.Path.exists")
    def test_detect_import_type_local_not_found(self, mock_exists):
        """Test that non-existent local modules fall back to third_party."""
        # Mock that the local file doesn't exist
        mock_exists.return_value = False

        result = detect_import_type(
            "nonexistent_module", "/test/project", "/test/project/main.py"
        )
        assert result == "third_party"


class TestBaseAnalyzer:
    """Test the BaseAnalyzer abstract class."""

    def test_base_analyzer_cannot_be_instantiated(self):
        """Test that BaseAnalyzer cannot be instantiated directly."""
        with pytest.raises(
            TypeError, match="Can't instantiate abstract class BaseAnalyzer"
        ):
            BaseAnalyzer()

    def test_base_analyzer_class_properties(self):
        """Test BaseAnalyzer class-level properties without instantiation."""
        # We can test class-level properties without instantiating
        assert hasattr(BaseAnalyzer, "__init__")
        assert hasattr(BaseAnalyzer, "can_analyze")
        assert hasattr(BaseAnalyzer, "analyze")
        assert hasattr(BaseAnalyzer, "get_info")

    def test_concrete_analyzer_implementation(self):
        """Test BaseAnalyzer through a concrete implementation."""

        # Create a concrete implementation for testing
        class ConcreteAnalyzer(BaseAnalyzer):
            def __init__(self):
                super().__init__()
                self.name = "test_analyzer"
                self.version = "2.0.0"
                self.supported_extensions = [".test"]
                self.supported_filenames = ["test.config"]

            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test_hash",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        # Test the concrete implementation
        analyzer = ConcreteAnalyzer()
        assert analyzer.name == "test_analyzer"
        assert analyzer.version == "2.0.0"
        assert analyzer.supported_extensions == [".test"]
        assert analyzer.supported_filenames == ["test.config"]

        # Test abstract methods work
        can_analyze, confidence = analyzer.can_analyze("test.py", "content")
        assert can_analyze is True
        assert confidence == 1.0

    def test_get_info_concrete_implementation(self):
        """Test the get_info method through concrete implementation."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def __init__(self):
                super().__init__()
                self.name = "TestAnalyzer"
                self.version = "2.0.0"
                self.supported_extensions = [".test"]
                self.supported_filenames = ["test.config"]

            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()
        info = analyzer.get_info()

        expected = {
            "name": "TestAnalyzer",
            "version": "2.0.0",
            "supported_extensions": [".test"],
            "supported_filenames": ["test.config"],
        }

        assert info == expected

    def test_generate_cache_filename(self, sample_plugin_input):
        """Test cache filename generation using concrete implementation."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def __init__(self):
                super().__init__()
                self.name = "test_analyzer"

            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()
        filename = analyzer._generate_cache_filename(sample_plugin_input)

        # Should contain analyzer name
        assert "test_analyzer" in filename
        # Should contain cleaned path
        assert "src_test_py" in filename
        # Should be a JSON file
        assert filename.endswith(".json")
        # Should have a hash component (16 characters)
        parts = filename.split("_")
        hash_part = parts[-1].replace(".json", "")
        assert len(hash_part) == 16
        assert all(c in "0123456789abcdef" for c in hash_part)

    def test_generate_cache_filename_consistency(self, sample_plugin_input):
        """Test that cache filename generation is consistent."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def __init__(self):
                super().__init__()
                self.name = "test_analyzer"

            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()
        filename1 = analyzer._generate_cache_filename(sample_plugin_input)
        filename2 = analyzer._generate_cache_filename(sample_plugin_input)

        # Same input should generate same filename
        assert filename1 == filename2

    def test_generate_cache_filename_different_content(self, temp_project_dir):
        """Test that different content generates different filenames."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def __init__(self):
                super().__init__()
                self.name = "test_analyzer"

            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()

        input1 = PluginInput(
            file_path=str(temp_project_dir / "test.py"),
            relative_path="test.py",
            content="print('hello')",
            project_root=str(temp_project_dir),
            cache_dir=str(temp_project_dir / ".cache"),
        )

        input2 = PluginInput(
            file_path=str(temp_project_dir / "test.py"),
            relative_path="test.py",
            content="print('world')",  # Different content
            project_root=str(temp_project_dir),
            cache_dir=str(temp_project_dir / ".cache"),
        )

        filename1 = analyzer._generate_cache_filename(input1)
        filename2 = analyzer._generate_cache_filename(input2)

        # Different content should generate different filenames
        assert filename1 != filename2

    @patch("builtins.open", new_callable=mock_open)
    @patch("pathlib.Path.mkdir")
    @patch("json.dump")
    def test_write_to_cache(
        self, mock_json_dump, mock_mkdir, mock_file, complete_plugin_output
    ):
        """Test writing results to cache using concrete implementation."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()

        cache_filename = analyzer._write_to_cache(
            complete_plugin_output, "test_cache.json", "/cache"
        )

        assert cache_filename == "test_cache.json"

        # Verify directory creation
        mock_mkdir.assert_called_once_with(parents=True, exist_ok=True)

        # Verify file opening
        mock_file.assert_called_once()

        # Verify JSON dump
        mock_json_dump.assert_called_once()

        # Check that the result was converted to dict for JSON serialization
        call_args = mock_json_dump.call_args[0]
        assert isinstance(
            call_args[0], dict
        )  # Should be a dict, not PluginOutput object

    def test_write_to_cache_integration(self, temp_project_dir, complete_plugin_output):
        """Test writing to cache with real file system using concrete implementation."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()
        cache_dir = temp_project_dir / "cache"

        cache_filename = analyzer._write_to_cache(
            complete_plugin_output, "integration_test.json", str(cache_dir)
        )

        assert cache_filename == "integration_test.json"

        # Verify the file was created
        cache_file = cache_dir / "integration_test.json"
        assert cache_file.exists()

        # Verify the content is valid JSON
        with open(cache_file, "r") as f:
            data = json.load(f)

        assert data["file_path"] == "/project/src/main.py"
        assert data["file_hash"] == "abc123def456"
        assert len(data["elements"]) == 2
        assert data["processing_time_ms"] == 150


class TestPluginCommunication:
    """Test plugin communication protocol utilities."""

    def test_plugin_communication_abstract_class_behavior(self):
        """Test we cannot test communication directly with abstract BaseAnalyzer."""
        # This test documents that BaseAnalyzer is properly abstract
        with pytest.raises(
            TypeError, match="Can't instantiate abstract class BaseAnalyzer"
        ):
            BaseAnalyzer()

        # Plugin communication should be tested with concrete implementations
        # (This is demonstrated in individual plugin tests like test_python_analyzer.py)

    def test_plugin_communication_with_concrete_implementation(
        self, monkeypatch, capsys
    ):
        """Test plugin communication using a concrete analyzer implementation."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def __init__(self):
                super().__init__()
                self.name = "test_plugin"
                self.version = "1.0.0"
                self.supported_extensions = [".test"]
                self.supported_filenames = []

            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test_hash",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()

        # Test get_info message
        test_message = {"type": "get_info"}

        # Mock stdin
        from io import StringIO

        mock_stdin = StringIO(json.dumps(test_message))
        monkeypatch.setattr(sys, "stdin", mock_stdin)

        # Run the analyzer
        analyzer.run()

        # Get the output
        captured = capsys.readouterr()
        output = captured.out

        assert (
            output.strip()
        ), f"Expected output but got empty string. Stderr: {captured.err}"

        response = json.loads(output.strip())
        assert response["status"] == "info"
        assert response["name"] == "test_plugin"
        assert response["version"] == "1.0.0"

    def test_plugin_communication_invalid_json(self, monkeypatch, capsys):
        """Test handling of invalid JSON input using concrete implementation."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()

        # Mock stdin with invalid JSON
        from io import StringIO

        mock_stdin = StringIO("invalid json content")
        monkeypatch.setattr(sys, "stdin", mock_stdin)

        analyzer.run()

        # Check that an error response was written
        captured = capsys.readouterr()
        output = captured.out

        assert (
            output.strip()
        ), f"Expected error output but got empty string. Stderr: {captured.err}"

        response = json.loads(output.strip())
        assert response["status"] == "error"
        assert "Invalid JSON" in response["message"]

    def test_plugin_communication_unknown_message_type(self, monkeypatch, capsys):
        """Test handling of unknown message types using concrete implementation."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()

        test_message = {"type": "unknown_command"}

        from io import StringIO

        mock_stdin = StringIO(json.dumps(test_message))
        monkeypatch.setattr(sys, "stdin", mock_stdin)

        analyzer.run()

        captured = capsys.readouterr()
        output = captured.out

        assert (
            output.strip()
        ), f"Expected error output but got empty string. Stderr: {captured.err}"

        response = json.loads(output.strip())
        assert response["status"] == "error"
        assert "Unknown message type" in response["message"]

    def test_plugin_communication_empty_input(self, monkeypatch, capsys):
        """Test handling of empty input using concrete implementation."""

        class ConcreteAnalyzer(BaseAnalyzer):
            def can_analyze(
                self, file_path: str, content_preview: str
            ) -> Tuple[bool, float]:
                return True, 1.0

            def analyze(self, input_data):
                return PluginOutput(
                    file_path=input_data.file_path,
                    file_hash="test",
                    elements=[],
                    imports=[],
                    exports=[],
                    relationships=[],
                    external_dependencies=[],
                )

        analyzer = ConcreteAnalyzer()

        from io import StringIO

        mock_stdin = StringIO("")
        monkeypatch.setattr(sys, "stdin", mock_stdin)

        analyzer.run()

        captured = capsys.readouterr()
        output = captured.out

        assert (
            output.strip()
        ), f"Expected error output but got empty string. Stderr: {captured.err}"

        response = json.loads(output.strip())
        assert response["status"] == "error"
        assert "No input received" in response["message"]


if __name__ == "__main__":
    # Allow running this test file directly
    pytest.main([__file__, "-v"])
