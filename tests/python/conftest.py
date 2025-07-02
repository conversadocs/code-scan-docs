"""
pytest configuration and fixtures for CSD plugin tests.

This file provides common fixtures and configuration that can be used
by all plugin tests, and serves as a reference for external plugin developers.
"""

import json
import sys
import tempfile
from pathlib import Path
from typing import Dict, Any
import pytest

# Add the plugins and shared directories to the path
# This mimics how external plugin developers would structure their imports
PROJECT_ROOT = Path(__file__).parent.parent.parent
PLUGINS_DIR = PROJECT_ROOT / "plugins"
SHARED_DIR = PLUGINS_DIR / "shared"

sys.path.insert(0, str(PLUGINS_DIR))
sys.path.insert(0, str(SHARED_DIR))


@pytest.fixture
def temp_project_dir():
    """Create a temporary directory that simulates a project structure."""
    with tempfile.TemporaryDirectory() as temp_dir:
        project_path = Path(temp_dir)

        # Create basic project structure
        (project_path / "src").mkdir()
        (project_path / ".csd_cache").mkdir()

        yield project_path


@pytest.fixture
def sample_plugin_input(temp_project_dir):
    """Create a sample PluginInput for testing."""
    from base_analyzer import PluginInput

    test_file = temp_project_dir / "src" / "test.py"
    test_file.write_text("def hello():\n    print('world')\n")

    return PluginInput(
        file_path=str(test_file),
        relative_path="src/test.py",
        content=test_file.read_text(),
        project_root=str(temp_project_dir),
        cache_dir=str(temp_project_dir / ".csd_cache"),
        plugin_config=None,
    )


@pytest.fixture
def sample_plugin_config():
    """Sample plugin configuration for testing."""
    return {"analyze_comments": True, "max_complexity": 10, "include_private": False}


@pytest.fixture
def sample_code_elements():
    """Sample code elements for testing."""
    from base_analyzer import CodeElement

    return [
        CodeElement(
            element_type="function",
            name="test_function",
            signature="def test_function(x: int) -> bool",
            line_start=1,
            line_end=5,
            summary="A test function",
            complexity_score=2,
            calls=["helper_function"],
            metadata={"is_async": False, "decorators": ["@pytest.fixture"]},
        ),
        CodeElement(
            element_type="class",
            name="TestClass",
            signature="class TestClass",
            line_start=7,
            line_end=15,
            summary="A test class",
            complexity_score=1,
            calls=["__init__", "method1"],
            metadata={"base_classes": [], "methods": ["__init__", "method1"]},
        ),
    ]


@pytest.fixture
def sample_imports():
    """Sample imports for testing."""
    from base_analyzer import Import

    return [
        Import(
            module="os", items=[], alias=None, line_number=1, import_type="standard"
        ),
        Import(
            module="mymodule",
            items=["function1", "Class1"],
            alias="mm",
            line_number=2,
            import_type="local",
        ),
        Import(
            module="requests",
            items=[],
            alias=None,
            line_number=3,
            import_type="third_party",
        ),
    ]


@pytest.fixture
def sample_relationships():
    """Sample relationships for testing."""
    from base_analyzer import Relationship

    return [
        Relationship(
            from_file="src/main.py",
            to_file="src/utils.py",
            relationship_type="import",
            details="imports utility functions",
            line_number=5,
            strength=0.8,
        ),
        Relationship(
            from_file="src/main.py",
            to_file="src/models.py",
            relationship_type="call",
            details="calls Model.create()",
            line_number=12,
            strength=0.6,
        ),
    ]


@pytest.fixture
def sample_external_dependencies():
    """Sample external dependencies for testing."""
    from base_analyzer import ExternalDependency

    return [
        ExternalDependency(
            name="requests",
            version="2.28.0",
            ecosystem="pip",
            dependency_type="runtime",
            source_file="requirements.txt",
        ),
        ExternalDependency(
            name="pytest",
            version="7.0.0",
            ecosystem="pip",
            dependency_type="development",
            source_file="requirements-dev.txt",
        ),
    ]


@pytest.fixture
def complete_plugin_output(
    sample_code_elements,
    sample_imports,
    sample_relationships,
    sample_external_dependencies,
):
    """A complete PluginOutput for testing."""
    from base_analyzer import PluginOutput

    return PluginOutput(
        file_path="/project/src/main.py",
        file_hash="abc123def456",
        elements=sample_code_elements,
        imports=sample_imports,
        exports=["main_function", "helper"],
        relationships=sample_relationships,
        external_dependencies=sample_external_dependencies,
        file_summary="Main application file with core functionality",
        processing_time_ms=150,
        plugin_version="1.0.0",
    )


# Utility functions for tests
def assert_valid_json_communication(output_str: str) -> Dict[str, Any]:
    """
    Helper function to validate JSON communication format.

    This is useful for testing the plugin communication protocol
    and can be used by external plugin developers.
    """
    try:
        data: Dict[str, Any] = json.loads(output_str.strip())
        assert "status" in data, "Response must have 'status' field"
        return data
    except json.JSONDecodeError as e:
        pytest.fail(f"Invalid JSON output: {e}\nOutput was: {output_str}")


def create_test_file(directory: Path, filename: str, content: str) -> Path:
    """
    Helper function to create test files.

    Args:
        directory: Directory to create file in
        filename: Name of file to create
        content: Content to write to file

    Returns:
        Path to created file
    """
    file_path = directory / filename
    file_path.parent.mkdir(parents=True, exist_ok=True)
    file_path.write_text(content)
    return file_path
