#!/usr/bin/env python3
"""
LLM utilities for CSD output plugins.
Provides shared LLM client functionality and token management.
"""

import aiohttp
import asyncio
import logging
import time
from typing import Dict, Any, Optional, List, cast
from dataclasses import dataclass


@dataclass
class LLMConfig:
    """Configuration for LLM integration."""

    provider: str = "ollama"
    base_url: str = "http://localhost:11434"
    model: str = "deepseek-coder:6.7b"
    timeout_seconds: int = 30
    max_tokens_per_request: int = 4000
    max_context_tokens: int = 2000
    temperature: float = 0.7
    top_p: float = 0.9


@dataclass
class LLMResponse:
    """Response from LLM."""

    content: str
    success: bool
    error: Optional[str] = None
    processing_time_ms: int = 0
    tokens_used: Optional[int] = None


class TokenManager:
    """Manages token estimation and budgeting for LLM requests."""

    def __init__(self, model_name: str):
        self.model_name = model_name
        self.chars_per_token = 4
        self.model_limits = {
            "deepseek-coder": {"max_total": 8192, "reserve_output": 1000},
            "deepseek-coder:6.7b": {"max_total": 8192, "reserve_output": 1000},
            "llama3": {"max_total": 4096, "reserve_output": 500},
            "default": {"max_total": 4096, "reserve_output": 500},
        }

    def estimate_tokens(self, text: str) -> int:
        """Estimate the number of tokens based on character count."""
        return len(text) // self.chars_per_token

    def get_model_limits(self) -> Dict[str, int]:
        """Get token limits for the current model."""
        for key in [self.model_name, self.model_name.split(":")[0], "default"]:
            if key in self.model_limits:
                return self.model_limits[key]
        return self.model_limits["default"]

    def calculate_available_context_tokens(
        self, system_prompt: str, user_prompt: str
    ) -> int:
        """Calculate how many tokens are available for context."""
        limits = self.get_model_limits()

        system_tokens = self.estimate_tokens(system_prompt)
        user_tokens = self.estimate_tokens(user_prompt)

        available = (
            limits["max_total"] - limits["reserve_output"] - system_tokens - user_tokens
        )
        return max(0, available)

    def trim_context(self, context: str, max_tokens: int) -> str:
        """Trim context to fit within token budget."""
        estimated_tokens = self.estimate_tokens(context)

        if estimated_tokens <= max_tokens:
            return context

        target_chars = max_tokens * self.chars_per_token

        if target_chars < len(context):
            trimmed = context[:target_chars]
            last_space = trimmed.rfind(" ")
            if last_space > target_chars * 0.8:
                trimmed = trimmed[:last_space]
            return trimmed + "... [content truncated for token limits]"

        return context


class LLMClient:
    """Client for interacting with LLM APIs."""

    def __init__(self, config: LLMConfig):
        self.config = config
        self.token_manager = TokenManager(config.model)
        self.logger = logging.getLogger(__name__)

    async def generate(
        self,
        prompt: str,
        system_prompt: Optional[str] = None,
        context: Optional[str] = None,
    ) -> LLMResponse:
        """Generate text using the LLM."""
        start_time = time.time()

        try:
            full_prompt = self._build_prompt(prompt, system_prompt, context)

            if self.config.provider.lower() == "ollama":
                response = await self._call_ollama(full_prompt)
            else:
                raise ValueError(f"Unsupported LLM provider: {self.config.provider}")

            processing_time = int((time.time() - start_time) * 1000)
            response.processing_time_ms = processing_time

            self.logger.info(f"LLM generation completed in {processing_time}ms")
            return response

        except Exception as e:
            processing_time = int((time.time() - start_time) * 1000)
            self.logger.error(f"LLM generation failed: {e}")

            return LLMResponse(
                content="",
                success=False,
                error=str(e),
                processing_time_ms=processing_time,
            )

    def _build_prompt(
        self,
        user_prompt: str,
        system_prompt: Optional[str] = None,
        context: Optional[str] = None,
    ) -> str:
        """Build the complete prompt with token management."""
        if system_prompt is None:
            system_prompt = (
                "You are a technical documentation expert. Generate clear, accurate, "
                "and helpful documentation based on code analysis. Be concise but thorough."
            )

        base_prompt = f"{system_prompt}\n\n{user_prompt}"
        available_tokens = self.token_manager.calculate_available_context_tokens(
            system_prompt, user_prompt
        )

        if context:
            trimmed_context = self.token_manager.trim_context(context, available_tokens)
            full_prompt = (
                f"{system_prompt}\n\nContext:\n{trimmed_context}\n\n{user_prompt}"
            )
        else:
            full_prompt = base_prompt

        self.logger.debug(
            f"Built prompt with ~{self.token_manager.estimate_tokens(full_prompt)} tokens"
        )
        return full_prompt

    async def _call_ollama(self, prompt: str) -> LLMResponse:
        """Make a request to Ollama API."""
        url = f"{self.config.base_url}/api/generate"

        payload = {
            "model": self.config.model,
            "prompt": prompt,
            "stream": False,
            "options": {
                "temperature": self.config.temperature,
                "top_p": self.config.top_p,
                "num_predict": self.config.max_tokens_per_request,
            },
        }

        timeout = aiohttp.ClientTimeout(total=self.config.timeout_seconds)

        try:
            async with aiohttp.ClientSession(timeout=timeout) as session:
                async with session.post(url, json=payload) as response:
                    if response.status == 200:
                        result = await response.json()

                        content = result.get("response", "").strip()

                        return LLMResponse(
                            content=content,
                            success=True,
                            tokens_used=result.get("total_duration", 0),
                        )
                    else:
                        error_text = await response.text()
                        return LLMResponse(
                            content="",
                            success=False,
                            error=f"HTTP {response.status}: {error_text}",
                        )

        except asyncio.TimeoutError:
            return LLMResponse(
                content="",
                success=False,
                error=f"Request timed out after {self.config.timeout_seconds} seconds",
            )
        except Exception as e:
            return LLMResponse(
                content="", success=False, error=f"Request failed: {str(e)}"
            )

    async def test_connection(self) -> bool:
        """Test if we can connect to the LLM."""
        try:
            if self.config.provider.lower() == "ollama":
                url = f"{self.config.base_url}/api/version"
                timeout = aiohttp.ClientTimeout(total=5)

                async with aiohttp.ClientSession(timeout=timeout) as session:
                    async with session.get(url) as response:
                        return response.status == 200
            return False
        except Exception:
            return False


class SectionProcessor:
    """Processes markdown sections with LLM enhancement."""

    def __init__(self, llm_client: LLMClient):
        self.llm_client = llm_client
        self.logger = logging.getLogger(__name__)

    def extract_sections(self, content: str) -> List[Dict[str, Any]]:
        """Extract sections marked with CSD comments."""
        sections = []
        lines = content.split("\n")

        i = 0
        while i < len(lines):
            line = lines[i].strip()

            if line.startswith("<!-- CSD:SECTION:"):
                section_name = (
                    line.replace("<!-- CSD:SECTION:", "").replace(" -->", "").strip()
                )
                start_line = i

                end_line = None
                content_lines = []

                for j in range(i + 1, len(lines)):
                    if lines[j].strip() == f"<!-- /CSD:SECTION:{section_name} -->":
                        end_line = j
                        break
                    content_lines.append(lines[j])

                if end_line is not None:
                    sections.append(
                        {
                            "name": section_name,
                            "start_line": start_line,
                            "end_line": end_line,
                            "original_content": "\n".join(content_lines),
                            "enhanced": False,
                        }
                    )
                    i = end_line

            i += 1

        self.logger.info(
            f"Found {len(sections)} sections to process: {[s['name'] for s in sections]}"
        )
        return sections

    async def enhance_section(
        self,
        section: Dict[str, Any],
        context: str,
        section_prompt: Optional[str] = None,
    ) -> str:
        """Enhance a single section with LLM."""
        if section_prompt is None:
            section_prompt = self._get_default_prompt(section["name"])

        full_context = f"Original content:\n{section['original_content']}\n\nProject context:\n{context}"

        response = await self.llm_client.generate(
            prompt=section_prompt, context=full_context
        )

        if response.success:
            self.logger.info(f"Successfully enhanced section '{section['name']}'")
            return response.content
        else:
            self.logger.error(
                f"Failed to enhance section '{section['name']}': {response.error}"
            )
            return cast(str, section["original_content"])

    def _get_default_prompt(self, section_name: str) -> str:
        """Get default prompt for a section type."""
        prompts = {
            "project_overview": (
                "Based on the project context and original content, write a clear "
                "project overview. Include what the project does and key features. "
                "Keep it concise (2-3 paragraphs maximum)."
            ),
            "installation": (
                "Based on the project dependencies and structure, write clear "
                "instructions. Include any prerequisites and basic setup steps."
            ),
            "api_reference": (
                "Based on the code structure, generate clean documentation. Include "
                "public functions and classes with their parameters and purpose."
            ),
            "usage": (
                "Write practical usage examples based on the project structure. "
                "Show how someone would typically use this project with examples."
            ),
            "examples": (
                "Create practical, working examples that demonstrate functionality. "
                "Base the examples on the actual code structure and public APIs."
            ),
        }

        return prompts.get(
            section_name,
            f"Enhance the content for the '{section_name}' section based on context. "
            "Make it clear, accurate, and helpful for users.",
        )

    def replace_section_content(
        self, original_content: str, section: Dict[str, Any], new_content: str
    ) -> str:
        """Replace a section's content in the original document."""
        lines = original_content.split("\n")

        new_lines = (
            lines[: section["start_line"] + 1]
            + [new_content]
            + lines[section["end_line"] :]
        )

        return "\n".join(new_lines)
