#!/usr/bin/env python3
"""
LLM-Enhanced Markdown Documentation Generator Plugin for CSD.
Generates comprehensive markdown documentation from project matrix with LLM enhancement.
"""

import sys
import asyncio
import logging
from pathlib import Path
from typing import Tuple, Dict, Any, Optional, List, cast

# Add the shared directory to the path so we can import base classes
sys.path.insert(0, str(Path(__file__).parent / "../shared"))

from csd_plugin_sdk import (
    BaseOutputPlugin,
    OutputPluginInput,
    OutputPluginResult,
    calculate_file_metrics,
    extract_dependencies,
)

from csd_plugin_sdk.utils import (
    LLMClient,
    LLMConfig,
    SectionProcessor,
)


class LLMMarkdownDocsPlugin(BaseOutputPlugin):
    """Generate LLM-enhanced Markdown documentation from project matrix."""

    def __init__(self):
        super().__init__()
        self.name = "llm_markdown_docs"
        self.version = "1.0.0"
        self.supported_output_types = ["documentation"]
        self.supported_formats = ["markdown"]

        # Set up logging
        self.logger = logging.getLogger(__name__)

    def can_generate(self, output_type: str, format: str) -> Tuple[bool, float]:
        """Check if this plugin can generate the requested output."""
        if output_type.lower() == "documentation" and format.lower() == "markdown":
            return True, 1.0
        return False, 0.0

    def generate(self, input_data: OutputPluginInput) -> OutputPluginResult:
        """Generate LLM-enhanced markdown documentation from the project matrix."""
        # Run the async generation in the event loop
        return asyncio.run(self._generate_async(input_data))

    async def _generate_async(
        self, input_data: OutputPluginInput
    ) -> OutputPluginResult:
        """Async implementation of documentation generation."""

        # Load the matrix data
        matrix_data = self._load_matrix_from_file(input_data.matrix_path)

        # Ensure output directory exists
        self._ensure_output_directory(input_data.output_dir)

        # Set up LLM configuration
        llm_config = self._create_llm_config(input_data.plugin_config)

        # Test LLM connection if configured
        llm_client = None
        if llm_config and self._should_use_llm(input_data.format_options):
            llm_client = LLMClient(llm_config)

            connection_ok = await llm_client.test_connection()
            if not connection_ok:
                self.logger.warning(
                    "LLM connection test failed, falling back to non-LLM generation"
                )
                llm_client = None
            else:
                self.logger.info(
                    f"LLM connection successful: {llm_config.provider} - {llm_config.model}"
                )

        # Generate documentation
        outputs = []

        # Look for existing template or use default
        template_content = await self._find_or_create_template(input_data, matrix_data)

        # Process the template
        if llm_client:
            enhanced_content = await self._enhance_with_llm(
                template_content, matrix_data, llm_client, input_data
            )
        else:
            enhanced_content = await self._generate_without_llm(
                template_content, matrix_data
            )

        # Save the enhanced documentation
        output_path = self._generate_output_filename(
            "documentation", "markdown", input_data.output_dir
        )

        with open(output_path, "w", encoding="utf-8") as f:
            f.write(enhanced_content)

        outputs.append(
            self._create_generated_output(
                output_path,
                "markdown",
                {
                    "llm_enhanced": llm_client is not None,
                    "template_source": (
                        "existing"
                        if await self._has_existing_template(input_data)
                        else "generated"
                    ),
                },
            )
        )

        return OutputPluginResult(
            plugin_name=self.name,
            plugin_version=self.version,
            output_type="documentation",
            outputs=outputs,
            metadata={
                "total_files_documented": len(matrix_data.get("files", {})),
                "llm_enhanced": llm_client is not None,
                "matrix_timestamp": matrix_data.get("metadata", {}).get(
                    "scan_timestamp"
                ),
                "model_used": llm_config.model if llm_config else None,
            },
        )

    def _create_llm_config(
        self, plugin_config: Optional[Dict[str, Any]]
    ) -> Optional[LLMConfig]:
        """Create LLM configuration from plugin config."""
        if not plugin_config:
            return None

        llm_section = plugin_config.get("llm", {})
        if not llm_section:
            return None

        return LLMConfig(
            provider=llm_section.get("provider", "ollama"),
            base_url=llm_section.get("base_url", "http://localhost:11434"),
            model=llm_section.get("model", "deepseek-coder:6.7b"),
            timeout_seconds=llm_section.get("timeout_seconds", 30),
            max_tokens_per_request=llm_section.get("max_tokens_per_request", 4000),
            max_context_tokens=llm_section.get("max_context_tokens", 2000),
            temperature=llm_section.get("temperature", 0.7),
            top_p=llm_section.get("top_p", 0.9),
        )

    def _should_use_llm(self, format_options: Dict[str, Any]) -> bool:
        """Check if LLM enhancement should be used."""
        return cast(bool, format_options.get("llm_enhance", True))

    async def _has_existing_template(self, input_data: OutputPluginInput) -> bool:
        """Check if there's an existing template to enhance."""
        readme_path = Path(input_data.project_root) / "README.md"
        return readme_path.exists()

    async def _find_or_create_template(
        self, input_data: OutputPluginInput, matrix_data: Dict[str, Any]
    ) -> str:
        """Find existing template or create a new one."""

        # Look for existing README.md
        readme_path = Path(input_data.project_root) / "README.md"
        if readme_path.exists():
            self.logger.info(f"Using existing README.md as template: {readme_path}")
            content = readme_path.read_text(encoding="utf-8")

            # If it already has CSD sections, use as-is
            if "<!-- CSD:SECTION:" in content:
                return content

            # Otherwise, wrap the entire content in a main section
            return self._wrap_existing_readme(content)

        # Create a new template
        self.logger.info("Creating new documentation template")
        return self._create_default_template(matrix_data)

    def _wrap_existing_readme(self, content: str) -> str:
        """Wrap existing README content with CSD section markers."""
        return f"""# Project Documentation

<!-- CSD:SECTION:project_overview -->
{content}
<!-- /CSD:SECTION:project_overview -->

<!-- CSD:SECTION:installation -->
## Installation

Installation instructions will be generated here.
<!-- /CSD:SECTION:installation -->

<!-- CSD:SECTION:usage -->
## Usage

Usage examples will be generated here.
<!-- /CSD:SECTION:usage -->

<!-- CSD:SECTION:api_reference -->
## API Reference

API documentation will be generated here.
<!-- /CSD:SECTION:api_reference -->
"""

    def _create_default_template(self, matrix_data: Dict[str, Any]) -> str:
        """Create a default documentation template."""
        project_name = Path(
            matrix_data.get("metadata", {}).get("project_root", "Project")
        ).name

        return f"""# {project_name}

<!-- CSD:SECTION:project_overview -->
A brief description of what this project does and its main purpose.
<!-- /CSD:SECTION:project_overview -->

## Installation

<!-- CSD:SECTION:installation -->
Installation instructions for this project.
<!-- /CSD:SECTION:installation -->

## Usage

<!-- CSD:SECTION:usage -->
Basic usage examples and getting started guide.
<!-- /CSD:SECTION:usage -->

## API Reference

<!-- CSD:SECTION:api_reference -->
Detailed API documentation and reference.
<!-- /CSD:SECTION:api_reference -->

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the terms specified in the LICENSE file.
"""

    async def _enhance_with_llm(
        self,
        template_content: str,
        matrix_data: Dict[str, Any],
        llm_client: LLMClient,
        input_data: OutputPluginInput,
    ) -> str:
        """Enhance template content using LLM."""

        processor = SectionProcessor(llm_client)
        sections = processor.extract_sections(template_content)

        if not sections:
            self.logger.warning(
                "No CSD sections found in template, using template as-is"
            )
            return template_content

        enhanced_content = template_content

        # Process each section
        for section in sections:
            self.logger.info(f"Processing section: {section['name']}")

            # Build context for this section
            context = self._build_section_context(section["name"], matrix_data)

            # Get custom prompt if configured
            section_prompt = self._get_section_prompt(
                section["name"], input_data.plugin_config
            )

            # Enhance the section
            try:
                enhanced_section_content = await processor.enhance_section(
                    section, context, section_prompt
                )

                # Replace in the full document
                enhanced_content = processor.replace_section_content(
                    enhanced_content, section, enhanced_section_content
                )

                self.logger.info(f"✅ Enhanced section: {section['name']}")

            except Exception as e:
                self.logger.error(f"❌ Failed to enhance section {section['name']}: {e}")
                # Continue with other sections
                continue

        return enhanced_content

    async def _generate_without_llm(
        self, template_content: str, matrix_data: Dict[str, Any]
    ) -> str:
        """Generate documentation without LLM enhancement (fallback)."""
        self.logger.info("Generating documentation without LLM enhancement")

        # For now, just fill sections with basic matrix data
        # In the future, we could have more sophisticated fallback generation

        processor = SectionProcessor(None)  # No LLM client
        sections = processor.extract_sections(template_content)

        if not sections:
            return template_content

        enhanced_content = template_content

        for section in sections:
            fallback_content = self._generate_fallback_content(
                section["name"], matrix_data
            )
            enhanced_content = processor.replace_section_content(
                enhanced_content, section, fallback_content
            )

        return enhanced_content

    def _build_section_context(
        self, section_name: str, matrix_data: Dict[str, Any]
    ) -> str:
        """Build relevant context for a specific section."""

        if section_name == "project_overview":
            return self._build_overview_context(matrix_data)
        elif section_name == "installation":
            return self._build_installation_context(matrix_data)
        elif section_name == "api_reference":
            return self._build_api_context(matrix_data)
        elif section_name == "usage":
            return self._build_usage_context(matrix_data)
        else:
            # Generic context
            return self._build_generic_context(matrix_data)

    def _build_overview_context(self, matrix_data: Dict[str, Any]) -> str:
        """Build context for project overview section."""
        context_parts = []

        # Project metadata
        metadata = matrix_data.get("metadata", {})
        context_parts.append(f"Project root: {metadata.get('project_root', 'Unknown')}")
        context_parts.append(
            f"Languages detected: {', '.join(metadata.get('plugins_used', []))}"
        )
        context_parts.append(f"Total files: {metadata.get('total_files', 0)}")

        # Dependencies
        deps = matrix_data.get("external_dependencies", [])
        if deps:
            context_parts.append(f"\nExternal dependencies ({len(deps)}):")
            for dep in deps[:10]:  # First 10 dependencies
                context_parts.append(
                    f"  - {dep.get('name', 'unknown')} ({dep.get('ecosystem', 'unknown')})"
                )

        # File structure overview
        files = matrix_data.get("files", {})
        if files:
            context_parts.append(f"\nKey files:")  # noqa: F541
            for file_path, file_info in list(files.items())[:5]:  # First 5 files
                context_parts.append(f"  - {file_path}")

        return "\n".join(context_parts)

    def _build_installation_context(self, matrix_data: Dict[str, Any]) -> str:
        """Build context for installation section."""
        context_parts = []

        # Dependencies by ecosystem
        deps_by_ecosystem = extract_dependencies(matrix_data)

        for ecosystem, deps in deps_by_ecosystem.items():
            context_parts.append(f"\n{ecosystem.upper()} dependencies:")
            for dep in deps[:10]:  # Limit to prevent token overflow
                name = dep.get("name", "unknown")
                version = dep.get("version", "")
                version_str = f" (version: {version})" if version else ""
                context_parts.append(f"  - {name}{version_str}")

        # Look for build/config files
        files = matrix_data.get("files", {})
        build_files = []
        for file_path, file_info in files.items():
            filename = Path(file_path).name.lower()
            if filename in [
                "package.json",
                "requirements.txt",
                "cargo.toml",
                "setup.py",
                "pyproject.toml",
                "makefile",
                "dockerfile",
            ]:
                build_files.append(file_path)

        if build_files:
            context_parts.append(f"\nBuild/config files found:")  # noqa: F541
            for bf in build_files:
                context_parts.append(f"  - {bf}")

        return "\n".join(context_parts)

    def _build_api_context(self, matrix_data: Dict[str, Any]) -> str:
        """Build context for API reference section."""
        context_parts = []

        # Public functions and classes
        files = matrix_data.get("files", {})
        public_elements = []

        for file_path, file_info in files.items():
            elements = file_info.get("elements", [])
            for element in elements:
                # Consider public if name doesn't start with _ or has public metadata
                name = element.get("name", "")
                metadata = element.get("metadata", {})
                is_public = (
                    not name.startswith("_")
                    or metadata.get("is_public", False)
                    or metadata.get("visibility") == "pub"
                )

                if is_public:
                    public_elements.append({"file": file_path, "element": element})

        # Group by element type
        by_type: Dict[str, List[Dict[str, Any]]] = {}
        for item in public_elements[:20]:  # Limit to prevent token overflow
            element_type = item["element"].get("element_type", "unknown")
            if element_type not in by_type:
                by_type[element_type] = []
            by_type[element_type].append(item)

        for element_type, items in by_type.items():
            context_parts.append(f"\n{element_type.title()}s:")
            for item in items[:5]:  # Limit per type
                element = item["element"]
                name = element.get("name", "unnamed")
                signature = element.get("signature", "")
                file_path = item["file"]

                context_parts.append(f"  - {name} in {file_path}")
                if signature:
                    context_parts.append(f"    Signature: {signature}")

        return "\n".join(context_parts)

    def _build_usage_context(self, matrix_data: Dict[str, Any]) -> str:
        """Build context for usage section."""
        # Similar to API context but focus on main entry points
        return self._build_api_context(matrix_data)

    def _build_generic_context(self, matrix_data: Dict[str, Any]) -> str:
        """Build generic context for unknown section types."""
        metrics = calculate_file_metrics(matrix_data)

        context_parts = [
            f"Project contains {metrics['total_files']} files",
            f"Total size: {metrics['total_size_mb']:.2f} MB",
            f"Languages: {', '.join(metrics['files_by_plugin'].keys())}",
            f"External dependencies: {len(matrix_data.get('external_dependencies', []))}",
        ]

        return "\n".join(context_parts)

    def _get_section_prompt(
        self, section_name: str, plugin_config: Optional[Dict[str, Any]]
    ) -> Optional[str]:
        """Get custom prompt for a section from plugin config."""
        if not plugin_config:
            return None

        prompts = plugin_config.get("prompts", {}).get("section_prompts", {})
        return cast(Optional[str], prompts.get(section_name))

    def _generate_fallback_content(
        self, section_name: str, matrix_data: Dict[str, Any]
    ) -> str:
        """Generate basic content for a section without LLM (fallback)."""

        if section_name == "project_overview":
            metadata = matrix_data.get("metadata", {})
            project_name = Path(metadata.get("project_root", "Project")).name
            languages = ", ".join(metadata.get("plugins_used", ["Multiple languages"]))
            total_files = metadata.get("total_files", 0)

            return f"""This project ({project_name}) contains {total_files} files written in {languages}.

The codebase includes {len(matrix_data.get("external_dependencies", []))} external dependencies and represents a {languages} project with a structured approach to development."""

        elif section_name == "installation":
            deps_by_ecosystem = extract_dependencies(matrix_data)
            install_commands = []

            for ecosystem, deps in deps_by_ecosystem.items():
                if ecosystem == "pip":
                    install_commands.append(
                        "```bash\npip install -r requirements.txt\n```"
                    )
                elif ecosystem == "cargo":
                    install_commands.append("```bash\ncargo build\n```")
                elif ecosystem == "npm":
                    install_commands.append("```bash\nnpm install\n```")

            if install_commands:
                return "## Installation\n\n" + "\n\n".join(install_commands)
            else:
                return "## Installation\n\nClone this repository and follow the setup instructions for your development environment."

        elif section_name == "api_reference":
            files = matrix_data.get("files", {})
            public_functions = []

            for file_path, file_info in files.items():
                elements = file_info.get("elements", [])
                for element in elements[:5]:  # Limit to prevent overwhelming output
                    if element.get("element_type") == "function" and not element.get(
                        "name", ""
                    ).startswith("_"):
                        public_functions.append(
                            f"- `{element.get('name', 'unknown')}` in {file_path}"
                        )

            if public_functions:
                return "## API Reference\n\nMain public functions:\n\n" + "\n".join(
                    public_functions[:10]
                )
            else:
                return "## API Reference\n\nAPI documentation is available in the source code."

        elif section_name == "usage":
            return """## Usage

```python
# Basic usage example
# (Update this with actual usage patterns from your codebase)
```

See the API reference for detailed function documentation."""

        else:
            return f"## {section_name.replace('_', ' ').title()}\n\nContent for this section will be added based on project analysis."


def main():
    """Main entry point for the plugin."""
    plugin = LLMMarkdownDocsPlugin()
    plugin.run()


if __name__ == "__main__":
    main()
