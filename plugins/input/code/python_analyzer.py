#!/usr/bin/env python3
"""
Python code analyzer plugin for CSD.
Analyzes Python files including .py files and Python ecosystem files.
"""

import ast
import io
import sys
import typing
import re
from pathlib import Path
from typing import List, Optional, Tuple
from csd_plugin_sdk import (
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

typing.cast(io.TextIOWrapper, sys.stdout).reconfigure(line_buffering=True)


class PythonAnalyzer(BaseAnalyzer):
    """Analyzer for Python files and Python ecosystem files."""

    def __init__(self):
        """Initialize the PythonAnalyzer instance."""
        super().__init__()
        self.name = "python"
        self.version = "1.0.0"
        self.supported_extensions = [".py"]
        self.supported_filenames = [
            "requirements.txt",
            "setup.py",
            "pyproject.toml",
            "Pipfile",
            "poetry.lock",
            "tox.ini",
            "pytest.ini",
            ".flake8",
            ".pylintrc",
        ]

    def can_analyze(self, file_path: str, content_preview: str) -> Tuple[bool, float]:
        """Check if this plugin can analyze the given file."""
        path = Path(file_path)

        if path.suffix in self.supported_extensions:
            return True, 1.0

        if path.name.lower() in [name.lower() for name in self.supported_filenames]:
            return True, 0.9

        if content_preview.startswith(
            "#!/usr/bin/env python"
        ) or content_preview.startswith("#!/usr/bin/python"):
            return True, 0.8

        python_indicators = ["def ", "class ", "import ", "from ", "__name__"]
        indicator_count = sum(
            1 for indicator in python_indicators if indicator in content_preview
        )

        if indicator_count >= 2:
            return True, 0.7
        elif indicator_count >= 1:
            return True, 0.5

        return False, 0.0

    def analyze(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a Python file."""
        file_path = Path(input_data.file_path)

        if file_path.suffix == ".py":
            return self._analyze_python_code(input_data)
        elif file_path.name.lower() == "requirements.txt":
            return self._analyze_requirements_txt(input_data)
        elif file_path.name.lower() == "setup.py":
            return self._analyze_setup_py(input_data)
        elif file_path.name.lower() == "pyproject.toml":
            return self._analyze_pyproject_toml(input_data)
        else:
            return self._analyze_python_code(input_data)

    def _analyze_python_code(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a .py file using AST parsing."""
        try:
            tree = ast.parse(input_data.content)
        except SyntaxError as e:
            return PluginOutput(
                file_path=input_data.file_path,
                file_hash="",
                elements=[],
                imports=[],
                exports=[],
                relationships=[],
                external_dependencies=[],
                file_summary=f"Syntax error in Python file: {e}",
            )

        elements = self._extract_elements(tree, input_data.content)
        imports = self._extract_imports(tree, input_data)
        exports = self._extract_exports(tree)
        relationships = self._extract_relationships(imports, input_data)

        return PluginOutput(
            file_path=input_data.file_path,
            file_hash="",
            elements=elements,
            imports=imports,
            exports=exports,
            relationships=relationships,
            external_dependencies=[],
        )

    def _extract_elements(self, tree: ast.AST, content: str) -> List[CodeElement]:
        """Extract code elements (functions, classes, etc.) from AST."""
        elements = []

        for node in ast.walk(tree):
            if isinstance(node, ast.FunctionDef):
                elements.append(self._create_function_element(node, content))
            elif isinstance(node, ast.AsyncFunctionDef):
                elements.append(self._create_async_function_element(node, content))
            elif isinstance(node, ast.ClassDef):
                elements.append(self._create_class_element(node, content))
            elif isinstance(node, (ast.Assign, ast.AnnAssign)) and hasattr(
                node, "lineno"
            ):
                var_element = self._create_variable_element(node, content)
                if var_element:
                    elements.append(var_element)

        return elements

    def _create_function_element(
        self, node: ast.FunctionDef, content: str
    ) -> CodeElement:
        """Create a CodeElement for a function."""
        args = [arg.arg for arg in node.args.args]
        signature = f"def {node.name}({', '.join(args)})"

        calls = self._extract_function_calls(node)

        complexity = calculate_complexity(
            content, node.lineno, node.end_lineno or node.lineno
        )

        decorators = [self._get_decorator_name(d) for d in node.decorator_list]

        return CodeElement(
            element_type="function",
            name=node.name,
            signature=signature,
            line_start=node.lineno,
            line_end=node.end_lineno or node.lineno,
            complexity_score=complexity,
            calls=calls,
            metadata={
                "is_async": False,
                "decorators": decorators,
                "arg_count": len(args),
                "has_docstring": ast.get_docstring(node) is not None,
            },
        )

    def _create_async_function_element(
        self, node: ast.AsyncFunctionDef, content: str
    ) -> CodeElement:
        """Create a CodeElement for an async function."""
        args = [arg.arg for arg in node.args.args]
        signature = f"async def {node.name}({', '.join(args)})"

        calls = self._extract_function_calls(node)
        complexity = calculate_complexity(
            content, node.lineno, node.end_lineno or node.lineno
        )
        decorators = [self._get_decorator_name(d) for d in node.decorator_list]

        return CodeElement(
            element_type="function",
            name=node.name,
            signature=signature,
            line_start=node.lineno,
            line_end=node.end_lineno or node.lineno,
            complexity_score=complexity,
            calls=calls,
            metadata={
                "is_async": True,
                "decorators": decorators,
                "arg_count": len(args),
                "has_docstring": ast.get_docstring(node) is not None,
            },
        )

    def _create_class_element(self, node: ast.ClassDef, content: str) -> CodeElement:
        """Create a CodeElement for a class."""
        bases = [self._get_name_from_node(base) for base in node.bases]
        signature = (
            f"class {node.name}({', '.join(bases)})" if bases else f"class {node.name}"
        )

        methods = []
        for item in node.body:
            if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                methods.append(item.name)

        decorators = [self._get_decorator_name(d) for d in node.decorator_list]

        return CodeElement(
            element_type="class",
            name=node.name,
            signature=signature,
            line_start=node.lineno,
            line_end=node.end_lineno or node.lineno,
            calls=methods,
            metadata={
                "base_classes": bases,
                "methods": methods,
                "decorators": decorators,
                "has_docstring": ast.get_docstring(node) is not None,
            },
        )

    def _create_variable_element(
        self, node: ast.AST, content: str
    ) -> Optional[CodeElement]:
        """Create a CodeElement for a module-level variable."""
        if isinstance(node, ast.Assign):
            if len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
                var_name = node.targets[0].id
                if not var_name.startswith("_") and var_name.isupper():
                    return CodeElement(
                        element_type="variable",
                        name=var_name,
                        line_start=node.lineno,
                        line_end=node.lineno,
                        metadata={"is_constant": True},
                    )
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            var_name = node.target.id
            if not var_name.startswith("_"):
                return CodeElement(
                    element_type="variable",
                    name=var_name,
                    line_start=node.lineno,
                    line_end=node.lineno,
                    metadata={"has_type_annotation": True},
                )

        return None

    def _extract_function_calls(self, node: ast.AST) -> List[str]:
        """Extract function calls from within a function or class."""
        calls = []

        for child in ast.walk(node):
            if isinstance(child, ast.Call):
                call_name = self._get_name_from_node(child.func)
                if call_name:
                    calls.append(call_name)

        return list(set(calls))

    def _extract_imports(self, tree: ast.AST, input_data: PluginInput) -> List[Import]:
        """Extract import statements from AST."""
        imports = []

        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    import_type = detect_import_type(
                        alias.name, input_data.project_root, input_data.file_path
                    )
                    imports.append(
                        Import(
                            module=alias.name,
                            alias=alias.asname,
                            line_number=node.lineno,
                            import_type=import_type,
                        )
                    )

            elif isinstance(node, ast.ImportFrom):
                module_name = node.module or ""
                items = [alias.name for alias in node.names]

                import_type = detect_import_type(
                    module_name, input_data.project_root, input_data.file_path
                )

                imports.append(
                    Import(
                        module=module_name,
                        items=items,
                        line_number=node.lineno,
                        import_type=import_type,
                    )
                )

        return imports

    def _extract_exports(self, tree: ast.Module) -> List[str]:
        """Extract what this module exports (defined at module level)."""
        exports = []

        for node in tree.body:
            if isinstance(node, ast.FunctionDef):
                if not node.name.startswith("_"):
                    exports.append(node.name)
            elif isinstance(node, ast.AsyncFunctionDef):
                if not node.name.startswith("_"):
                    exports.append(node.name)
            elif isinstance(node, ast.ClassDef):
                if not node.name.startswith("_"):
                    exports.append(node.name)
            elif isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name) and not target.id.startswith("_"):
                        exports.append(target.id)

        return exports

    def _extract_relationships(
        self, imports: List[Import], input_data: PluginInput
    ) -> List[Relationship]:
        """Extract file relationships based on imports."""
        relationships = []

        for imp in imports:
            if imp.import_type == "local":
                target_file = self._resolve_import_path(imp.module, input_data)
                if target_file:
                    relationships.append(
                        Relationship(
                            from_file=input_data.relative_path,
                            to_file=target_file,
                            relationship_type="import",
                            details=f"import {imp.module}",
                            line_number=imp.line_number,
                            strength=0.8,
                        )
                    )

        return relationships

    def _resolve_import_path(
        self, module_name: str, input_data: PluginInput
    ) -> Optional[str]:
        """Try to resolve a local import to an actual file path."""
        project_root = Path(input_data.project_root)
        current_dir = Path(input_data.file_path).parent

        module_path = module_name.replace(".", "/")

        potential_paths = [
            project_root / f"{module_path}.py",
            project_root / f"{module_path}/__init__.py",
            current_dir / f"{module_path}.py",
            current_dir / f"{module_path}/__init__.py",
        ]

        for path in potential_paths:
            if path.exists():
                try:
                    return str(path.relative_to(project_root))
                except ValueError:
                    return str(path)

        return None

    def _analyze_requirements_txt(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a requirements.txt file."""
        dependencies = []

        for line_num, line in enumerate(input_data.content.split("\n"), 1):
            line = line.strip()
            if not line or line.startswith("#"):
                continue

            if "==" in line:
                name, version = line.split("==", 1)
                name = name.strip()
                version = version.strip()
            elif ">=" in line:
                name = line.split(">=")[0].strip()
                version = None
            elif "<=" in line:
                name = line.split("<=")[0].strip()
                version = None
            else:
                name = line.strip()
                version = None

            if "[" in name:
                name = name.split("[")[0]

            dependencies.append(
                ExternalDependency(
                    name=name,
                    version=version,
                    ecosystem="pip",
                    dependency_type="runtime",
                    source_file=input_data.relative_path,
                )
            )

        return PluginOutput(
            file_path=input_data.file_path,
            file_hash="",
            elements=[],
            imports=[],
            exports=[],
            relationships=[],
            external_dependencies=dependencies,
            file_summary=f"Python requirements with {len(dependencies)} dependencies",
        )

    def _analyze_setup_py(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a setup.py file."""
        result = self._analyze_python_code(input_data)

        dependencies = self._extract_setup_dependencies(input_data.content)
        result.external_dependencies.extend(dependencies)
        result.file_summary = f"Python setup file with {len(dependencies)} dependencies"

        return result

    def _extract_setup_dependencies(self, content: str) -> List[ExternalDependency]:
        """Extract dependencies from setup.py content."""
        dependencies = []

        patterns = [
            r"install_requires\s*=\s*\[(.*?)\]",
            r"requires\s*=\s*\[(.*?)\]",
        ]

        for pattern in patterns:
            matches = re.findall(pattern, content, re.DOTALL)
            for match in matches:
                deps = re.findall(r'["\']([^"\']+)["\']', match)
                for dep in deps:
                    if ">=" in dep:
                        name = dep.split(">=")[0].strip()
                    elif "==" in dep:
                        name, version = dep.split("==", 1)
                        name = name.strip()
                    else:
                        name = dep.strip()

                    dependencies.append(
                        ExternalDependency(
                            name=name,
                            ecosystem="pip",
                            dependency_type="runtime",
                            source_file="setup.py",
                        )
                    )

        return dependencies

    def _analyze_pyproject_toml(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a pyproject.toml file."""
        dependencies = []

        lines = input_data.content.split("\n")
        in_dependencies = False

        for line in lines:
            line = line.strip()

            if "[tool.poetry.dependencies]" in line or "[project.dependencies]" in line:
                in_dependencies = True
                continue
            elif line.startswith("[") and in_dependencies:
                in_dependencies = False
                continue

            if in_dependencies and "=" in line and not line.startswith("#"):
                parts = line.split("=", 1)
                if len(parts) == 2:
                    name = parts[0].strip()
                    name = name.strip("\"'")

                    if name != "python":
                        dependencies.append(
                            ExternalDependency(
                                name=name,
                                ecosystem="pip",
                                dependency_type="runtime",
                                source_file=input_data.relative_path,
                            )
                        )

        return PluginOutput(
            file_path=input_data.file_path,
            file_hash="",
            elements=[],
            imports=[],
            exports=[],
            relationships=[],
            external_dependencies=dependencies,
            file_summary=f"Python project config with {len(dependencies)} dependencies",
        )

    def _get_decorator_name(self, decorator: ast.AST) -> str:
        """Get the name of a decorator."""
        return self._get_name_from_node(decorator) or "unknown"

    def _get_name_from_node(self, node: ast.AST) -> Optional[str]:
        """Extract name from various AST node types."""
        if isinstance(node, ast.Name):
            return node.id
        elif isinstance(node, ast.Attribute):
            value = self._get_name_from_node(node.value)
            return f"{value}.{node.attr}" if value else node.attr
        elif isinstance(node, ast.Call):
            return self._get_name_from_node(node.func)
        elif isinstance(node, ast.Constant) and isinstance(node.value, str):
            return node.value
        elif isinstance(node, ast.Str):
            return node.s
        else:
            return None


def main():
    """Main entry point for the plugin."""
    analyzer = PythonAnalyzer()
    analyzer.run()


if __name__ == "__main__":
    main()
