#!/usr/bin/env python3
"""
Rust code analyzer plugin for CSD.
Analyzes Rust files including .rs files and Rust ecosystem files.
"""

import re
import sys
from pathlib import Path
from typing import List, Dict, Any, Optional, Set, Tuple, Union

# Add the shared directory to the path so we can import base_analyzer
sys.path.insert(0, str(Path(__file__).parent / "shared"))

from base_analyzer import (
    BaseAnalyzer,
    CodeElement,
    Import,
    Relationship,
    ExternalDependency,
    PluginInput,
    PluginOutput,
    calculate_complexity,
)


class RustAnalyzer(BaseAnalyzer):
    """Analyzer for Rust files and Rust ecosystem files."""

    def __init__(self):
        super().__init__()
        self.name = "rust"
        self.version = "1.0.0"
        self.supported_extensions = [".rs"]
        self.supported_filenames = [
            "Cargo.toml",
            "Cargo.lock",
            ".rustfmt.toml",
            "rust-toolchain.toml",
            "rust-toolchain",
        ]

    def can_analyze(self, file_path: str, content_preview: str) -> Tuple[bool, float]:
        """Check if this plugin can analyze the given file."""
        path = Path(file_path)

        # Check by extension
        if path.suffix in self.supported_extensions:
            return True, 1.0

        # Check by filename
        if path.name.lower() in [name.lower() for name in self.supported_filenames]:
            return True, 0.9

        # Check content for Rust-specific keywords
        rust_indicators = [
            "fn ",
            "struct ",
            "impl ",
            "enum ",
            "trait ",
            "mod ",
            "use ",
            "pub ",
        ]
        indicator_count = sum(
            1 for indicator in rust_indicators if indicator in content_preview
        )

        if indicator_count >= 3:
            return True, 0.8
        elif indicator_count >= 1:
            return True, 0.6

        return False, 0.0

    def analyze(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a Rust file."""
        file_path = Path(input_data.file_path)

        # Handle different file types
        if file_path.suffix == ".rs":
            return self._analyze_rust_code(input_data)
        elif file_path.name == "Cargo.toml":
            return self._analyze_cargo_toml(input_data)
        elif file_path.name == "Cargo.lock":
            return self._analyze_cargo_lock(input_data)
        else:
            # Try to analyze as Rust code
            return self._analyze_rust_code(input_data)

    def _analyze_rust_code(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a .rs file using regex parsing."""
        content = input_data.content
        lines = content.split("\n")

        # Extract elements
        elements = self._extract_elements(content, lines)
        imports = self._extract_imports(content, lines, input_data)
        exports = self._extract_exports(content, lines)
        relationships = self._extract_relationships(imports, input_data)

        return PluginOutput(
            file_path=input_data.file_path,
            file_hash="",  # Will be filled by core
            elements=elements,
            imports=imports,
            exports=exports,
            relationships=relationships,
            external_dependencies=[],  # No external deps from .rs files directly
        )

    def _extract_elements(self, content: str, lines: List[str]) -> List[CodeElement]:
        """Extract code elements (functions, structs, etc.) from Rust code."""
        elements = []

        # Patterns for different Rust constructs
        patterns = {
            "function": [
                r"^\s*(pub\s+)?fn\s+(\w+)",
                r"^\s*(pub\s+)?(async\s+)?fn\s+(\w+)",
            ],
            "struct": [
                r"^\s*(pub\s+)?struct\s+(\w+)",
            ],
            "enum": [
                r"^\s*(pub\s+)?enum\s+(\w+)",
            ],
            "trait": [
                r"^\s*(pub\s+)?trait\s+(\w+)",
            ],
            "impl": [
                r"^\s*impl(?:\s*<[^>]*>)?\s+(\w+)",
                r"^\s*impl(?:\s*<[^>]*>)?\s+\w+\s+for\s+(\w+)",
            ],
            "module": [
                r"^\s*(pub\s+)?mod\s+(\w+)",
            ],
            "type": [
                r"^\s*(pub\s+)?type\s+(\w+)",
            ],
            "constant": [
                r"^\s*(pub\s+)?const\s+(\w+)",
            ],
        }

        for line_num, line in enumerate(lines, 1):
            line_stripped = line.strip()

            # Skip comments and empty lines
            if not line_stripped or line_stripped.startswith("//"):
                continue

            for element_type, type_patterns in patterns.items():
                for pattern in type_patterns:
                    match = re.match(pattern, line)
                    if match:
                        # Extract the name (usually the last capture group)
                        name = match.groups()[-1]

                        # Find the end of this element (simplified)
                        end_line = self._find_element_end(
                            lines, line_num - 1, line_stripped
                        )

                        # Calculate complexity
                        complexity = calculate_complexity(content, line_num, end_line)

                        # Extract function calls within this element
                        calls = self._extract_calls_in_range(
                            lines, line_num - 1, end_line - 1
                        )

                        # Check for visibility and other modifiers
                        is_public = "pub " in line
                        is_async = "async " in line and element_type == "function"

                        elements.append(
                            CodeElement(
                                element_type=element_type,
                                name=name,
                                signature=line_stripped,
                                line_start=line_num,
                                line_end=end_line,
                                complexity_score=complexity,
                                calls=calls,
                                metadata={
                                    "is_public": is_public,
                                    "is_async": (
                                        is_async
                                        if element_type == "function"
                                        else False
                                    ),
                                    "visibility": "pub" if is_public else "private",
                                },
                            )
                        )
                        break

        return elements

    def _find_element_end(
        self, lines: List[str], start_line: int, start_line_content: str
    ) -> int:
        """Find the end line of a Rust element (simplified brace matching)."""
        # If it's a single-line declaration, return the same line
        if start_line_content.endswith(";"):
            return start_line + 1

        brace_count = 0
        in_element = False

        for i, line in enumerate(lines[start_line:], start_line):
            # Count braces
            open_braces = line.count("{")
            close_braces = line.count("}")

            if open_braces > 0:
                in_element = True
                brace_count += open_braces

            brace_count -= close_braces

            # If we're in an element and braces are balanced, we've found the end
            if in_element and brace_count <= 0:
                return i + 1

        # If we couldn't find the end, estimate
        return min(start_line + 20, len(lines))

    def _extract_calls_in_range(
        self, lines: List[str], start: int, end: int
    ) -> List[str]:
        """Extract function/method calls within a range of lines."""
        calls: Set[str] = set()

        # Simple patterns for function calls
        call_patterns = [
            r"(\w+)\s*\(",  # Simple function calls
            r"\.(\w+)\s*\(",  # Method calls
            r"(\w+)::\w+\s*\(",  # Associated function calls
        ]

        for line_num in range(start, min(end, len(lines))):
            line = lines[line_num]

            for pattern in call_patterns:
                matches = re.findall(pattern, line)
                for match in matches:
                    if isinstance(match, tuple):
                        calls.update(match)
                    else:
                        calls.add(match)

        # Filter out common keywords and operators
        keywords = {
            "if",
            "else",
            "while",
            "for",
            "match",
            "let",
            "mut",
            "return",
            "break",
            "continue",
        }
        return [call for call in calls if call not in keywords and len(call) > 1]

    def _extract_imports(
        self, content: str, lines: List[str], input_data: PluginInput
    ) -> List[Import]:
        """Extract use statements from Rust code."""
        imports = []

        use_patterns = [
            r"^\s*use\s+([^;]+);",  # Standard use statements
            r"^\s*extern\s+crate\s+(\w+)",  # External crates (older style)
        ]

        for line_num, line in enumerate(lines, 1):
            for pattern in use_patterns:
                match = re.search(pattern, line)
                if match:
                    use_statement = match.group(1).strip()

                    # Parse the use statement
                    module, items = self._parse_use_statement(use_statement)

                    # Determine import type
                    import_type = self._determine_rust_import_type(
                        module, input_data.project_root
                    )

                    imports.append(
                        Import(
                            module=module,
                            items=items,
                            line_number=line_num,
                            import_type=import_type,
                        )
                    )

        return imports

    def _parse_use_statement(self, use_statement: str) -> Tuple[str, List[str]]:
        """Parse a Rust use statement into module and items."""
        # Handle different use statement formats:
        # use std::collections::HashMap;
        # use std::collections::{HashMap, HashSet};
        # use crate::module::*;
        # use super::module;
        # use self::module;

        if "::" not in use_statement:
            return use_statement, []

        if "{" in use_statement and "}" in use_statement:
            # Multiple imports: use std::collections::{HashMap, HashSet};
            parts = use_statement.split("{")
            module = parts[0].rstrip(":").strip()
            items_part = parts[1].split("}")[0]
            items = [item.strip() for item in items_part.split(",") if item.strip()]
            return module, items
        elif use_statement.endswith("::*"):
            # Glob import: use std::collections::*;
            module = use_statement[:-3]
            return module, ["*"]
        else:
            # Single import: use std::collections::HashMap;
            parts = use_statement.split("::")
            if len(parts) > 1:
                module = "::".join(parts[:-1])
                item = parts[-1]
                return module, [item]
            else:
                return use_statement, []

    def _determine_rust_import_type(self, module: str, project_root: str) -> str:
        """Determine the type of Rust import."""
        if module.startswith("crate::"):
            return "local"
        elif module.startswith("super::") or module.startswith("self::"):
            return "relative"
        elif (
            module.startswith("std::")
            or module.startswith("core::")
            or module.startswith("alloc::")
        ):
            return "standard"
        else:
            # Check if it's a local module by looking for mod.rs or [module].rs
            project_path = Path(project_root)
            module_parts = module.split("::")

            # Try to find local module files
            potential_paths = [
                project_path / "src" / f"{module_parts[0]}.rs",
                project_path / "src" / module_parts[0] / "mod.rs",
            ]

            for path in potential_paths:
                if path.exists():
                    return "local"

            return "third_party"

    def _extract_exports(self, content: str, lines: List[str]) -> List[str]:
        """Extract public items that this module exports."""
        exports = []

        # Patterns for public items
        pub_patterns = [
            r"^\s*pub\s+fn\s+(\w+)",
            r"^\s*pub\s+struct\s+(\w+)",
            r"^\s*pub\s+enum\s+(\w+)",
            r"^\s*pub\s+trait\s+(\w+)",
            r"^\s*pub\s+type\s+(\w+)",
            r"^\s*pub\s+const\s+(\w+)",
            r"^\s*pub\s+mod\s+(\w+)",
        ]

        for line in lines:
            for pattern in pub_patterns:
                match = re.search(pattern, line)
                if match:
                    exports.append(match.group(1))

        return list(set(exports))  # Remove duplicates

    def _extract_relationships(
        self, imports: List[Import], input_data: PluginInput
    ) -> List[Relationship]:
        """Extract file relationships based on imports."""
        relationships = []

        for imp in imports:
            if imp.import_type == "local":
                # Try to resolve the actual file path
                target_file = self._resolve_rust_module_path(imp.module, input_data)
                if target_file:
                    relationships.append(
                        Relationship(
                            from_file=input_data.relative_path,
                            to_file=target_file,
                            relationship_type="import",
                            details=f"use {imp.module}",
                            line_number=imp.line_number,
                            strength=0.8,
                        )
                    )

        return relationships

    def _resolve_rust_module_path(
        self, module_name: str, input_data: PluginInput
    ) -> Optional[str]:
        """Try to resolve a local Rust module to an actual file path."""
        project_root = Path(input_data.project_root)

        # Remove 'crate::' prefix if present
        if module_name.startswith("crate::"):
            module_name = module_name[7:]

        # Convert module path to file system path
        module_parts = module_name.split("::")

        # Try different locations
        potential_paths = [
            project_root / "src" / f"{'/'.join(module_parts)}.rs",
            project_root / "src" / f"{'/'.join(module_parts)}" / "mod.rs",
            project_root / "src" / f"{module_parts[0]}.rs",
            project_root / "src" / module_parts[0] / "mod.rs",
        ]

        for path in potential_paths:
            if path.exists():
                try:
                    return str(path.relative_to(project_root))
                except ValueError:
                    return str(path)

        return None

    def simple_toml_parse(self, content: str) -> Dict[str, Any]:
        """
        Simple TOML parser for basic dependency extraction.
        Only handles the subset of TOML we need for Cargo.toml files.
        """
        result: Dict[str, Dict[str, Any]] = {}
        current_section = None

        for line in content.split("\n"):
            line = line.strip()

            # Skip empty lines and comments
            if not line or line.startswith("#"):
                continue

            # Section headers
            if line.startswith("[") and line.endswith("]"):
                section_name = line[1:-1].strip()
                current_section = section_name
                if current_section not in result:
                    result[current_section] = {}
                continue

            # Key-value pairs
            if "=" in line and current_section:
                try:
                    key, value_raw = line.split("=", 1)
                    key = key.strip().strip('"')
                    value_raw = value_raw.strip()

                    value: Union[str, Dict[str, str]]
                    # Clean up the value
                    if value_raw.startswith('"') and value_raw.endswith('"'):
                        value = value_raw[1:-1]
                    elif value_raw.startswith("'") and value_raw.endswith("'"):
                        value = value_raw[1:-1]
                    elif value_raw.startswith("{") and value_raw.endswith("}"):
                        # Simple dict parsing for inline tables
                        dict_content = value_raw[1:-1]
                        parsed_dict: Dict[str, str] = {}
                        for pair in dict_content.split(","):
                            if "=" in pair:
                                k, v = pair.split("=", 1)
                                k = k.strip().strip('"')
                                v = v.strip().strip('"')
                                parsed_dict[k] = v
                        value = parsed_dict
                    else:
                        value = value_raw

                    result[current_section][key] = value

                except ValueError:
                    # Skip malformed lines
                    continue

        return result

    def _analyze_cargo_toml(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a Cargo.toml file."""
        dependencies = []

        try:
            cargo_data = self.simple_toml_parse(input_data.content)

            # Extract dependencies
            sections = ["dependencies", "dev-dependencies", "build-dependencies"]

            for section in sections:
                if section in cargo_data:
                    dep_type = {
                        "dependencies": "runtime",
                        "dev-dependencies": "development",
                        "build-dependencies": "build",
                    }.get(section, "runtime")

                    for name, spec in cargo_data[section].items():
                        version = None

                        if isinstance(spec, str):
                            version = spec
                        elif isinstance(spec, dict) and "version" in spec:
                            version = spec["version"]

                        dependencies.append(
                            ExternalDependency(
                                name=name,
                                version=version,
                                ecosystem="cargo",
                                dependency_type=dep_type,
                                source_file=input_data.relative_path,
                            )
                        )

        except Exception:
            # If TOML parsing fails, return basic info
            pass

        return PluginOutput(
            file_path=input_data.file_path,
            file_hash="",
            elements=[],
            imports=[],
            exports=[],
            relationships=[],
            external_dependencies=dependencies,
            file_summary=f"Rust Cargo.toml with {len(dependencies)} dependencies",
        )

    def _analyze_cargo_lock(self, input_data: PluginInput) -> PluginOutput:
        """Analyze a Cargo.lock file."""
        dependencies = []

        try:
            lock_data = self.simple_toml_parse(input_data.content)

            if "package" in lock_data:
                # Cargo.lock has a different format - packages are in an array
                # For simplicity, we'll just count them or extract basic info
                package_section = lock_data["package"]
                if isinstance(package_section, dict):
                    # Simple case - treat as single package info
                    name = package_section.get("name", "unknown")
                    version = package_section.get("version")

                    dependencies.append(
                        ExternalDependency(
                            name=name,
                            version=version,
                            ecosystem="cargo",
                            dependency_type="runtime",
                            source_file=input_data.relative_path,
                        )
                    )

        except Exception:
            # If TOML parsing fails, return basic info
            pass

        return PluginOutput(
            file_path=input_data.file_path,
            file_hash="",
            elements=[],
            imports=[],
            exports=[],
            relationships=[],
            external_dependencies=dependencies,
            file_summary=f"Rust Cargo.lock with {len(dependencies)} locked",
        )


def main():
    """Main entry point for the plugin."""
    analyzer = RustAnalyzer()
    analyzer.run()


if __name__ == "__main__":
    main()
