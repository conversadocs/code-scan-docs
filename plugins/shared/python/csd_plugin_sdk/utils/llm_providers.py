#!/usr/bin/env python3
"""
LLM Provider abstraction for CSD plugins.
Provides a common interface for different LLM services.
"""

import aiohttp
import asyncio
import logging
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Optional, Dict, Any


@dataclass
class LLMResponse:
    """Response from LLM."""

    content: str
    success: bool
    error: Optional[str] = None
    processing_time_ms: int = 0
    tokens_used: Optional[int] = None


class LLMProvider(ABC):
    """Abstract base class for LLM providers."""

    @abstractmethod
    async def generate(
        self,
        prompt: str,
        model: str,
        temperature: float = 0.7,
        max_tokens: int = 4000,
        top_p: float = 0.9,
        **kwargs,
    ) -> LLMResponse:
        """
        Generate text from the LLM.

        Args:
            prompt: The input prompt
            model: Model identifier (provider-specific)
            temperature: Sampling temperature
            max_tokens: Maximum tokens to generate
            top_p: Top-p sampling parameter
            **kwargs: Additional provider-specific parameters

        Returns:
            LLMResponse with generated content
        """
        pass

    @abstractmethod
    async def test_connection(self) -> bool:
        """
        Test if the provider is accessible.

        Returns:
            True if connection successful, False otherwise
        """
        pass

    def estimate_tokens(self, text: str) -> int:
        """
        Estimate the number of tokens in text.
        Can be overridden by providers with specific tokenizers.

        Args:
            text: Input text

        Returns:
            Estimated token count
        """
        return max(1, len(text) // 4)

    def get_model_limits(self, model: str) -> Dict[str, int]:
        """
        Get token limits for a specific model.
        Can be overridden by providers.

        Args:
            model: Model identifier

        Returns:
            Dict with 'max_total' and 'reserve_output' keys
        """
        return {"max_total": 4096, "reserve_output": 500}


class OllamaProvider(LLMProvider):
    """Ollama-specific LLM provider implementation."""

    def __init__(
        self, base_url: str = "http://localhost:11434", timeout_seconds: int = 30
    ):
        """
        Initialize Ollama provider.

        Args:
            base_url: Ollama API base URL
            timeout_seconds: Request timeout in seconds
        """
        self.base_url = base_url.rstrip("/")
        self.timeout_seconds = timeout_seconds
        self.logger = logging.getLogger(__name__)

        self.model_limits = {
            "deepseek-coder": {"max_total": 8192, "reserve_output": 1000},
            "deepseek-coder:6.7b": {"max_total": 8192, "reserve_output": 1000},
            "llama3": {"max_total": 4096, "reserve_output": 500},
            "codellama": {"max_total": 4096, "reserve_output": 500},
            "mistral": {"max_total": 8192, "reserve_output": 1000},
            "default": {"max_total": 4096, "reserve_output": 500},
        }

    async def generate(
        self,
        prompt: str,
        model: str,
        temperature: float = 0.7,
        max_tokens: int = 4000,
        top_p: float = 0.9,
        **kwargs,
    ) -> LLMResponse:
        """Generate text using Ollama API."""
        start_time = time.time()

        url = f"{self.base_url}/api/generate"

        payload: Dict[str, Any] = {
            "model": model,
            "prompt": prompt,
            "stream": False,
            "options": {
                "temperature": temperature,
                "top_p": top_p,
                "num_predict": max_tokens,
            },
        }

        if "seed" in kwargs:
            payload["options"]["seed"] = kwargs["seed"]
        if "stop" in kwargs:
            payload["options"]["stop"] = kwargs["stop"]

        timeout = aiohttp.ClientTimeout(total=self.timeout_seconds)

        try:
            async with aiohttp.ClientSession(timeout=timeout) as session:
                async with session.post(url, json=payload) as response:
                    processing_time = int((time.time() - start_time) * 1000)

                    if response.status == 200:
                        result = await response.json()

                        content = result.get("response", "").strip()

                        return LLMResponse(
                            content=content,
                            success=True,
                            tokens_used=result.get("total_duration", 0),
                            processing_time_ms=processing_time,
                        )
                    else:
                        error_text = await response.text()
                        return LLMResponse(
                            content="",
                            success=False,
                            error=f"HTTP {response.status}: {error_text}",
                            processing_time_ms=processing_time,
                        )

        except asyncio.TimeoutError:
            processing_time = int((time.time() - start_time) * 1000)
            return LLMResponse(
                content="",
                success=False,
                error=f"Request timed out after {self.timeout_seconds} seconds",
                processing_time_ms=processing_time,
            )
        except Exception as e:
            processing_time = int((time.time() - start_time) * 1000)
            return LLMResponse(
                content="",
                success=False,
                error=f"Request failed: {str(e)}",
                processing_time_ms=processing_time,
            )

    async def test_connection(self) -> bool:
        """Test connection to Ollama API."""
        try:
            url = f"{self.base_url}/api/version"
            timeout = aiohttp.ClientTimeout(total=5)

            async with aiohttp.ClientSession(timeout=timeout) as session:
                async with session.get(url) as response:
                    return response.status == 200
        except Exception as e:
            self.logger.debug(f"Ollama connection test failed: {e}")
            return False

    def get_model_limits(self, model: str) -> Dict[str, int]:
        """Get token limits for Ollama models."""
        if model in self.model_limits:
            return self.model_limits[model]

        base_model = model.split(":")[0] if ":" in model else model
        if base_model in self.model_limits:
            return self.model_limits[base_model]

        return self.model_limits["default"]


class OpenAIProvider(LLMProvider):
    """OpenAI provider stub for future implementation."""

    def __init__(
        self,
        api_key: str,
        organization: Optional[str] = None,
        timeout_seconds: int = 30,
    ):
        """
        Initialize OpenAI provider.

        Args:
            api_key: OpenAI API key
            organization: Optional organization ID
            timeout_seconds: Request timeout
        """
        self.api_key = api_key
        self.organization = organization
        self.timeout_seconds = timeout_seconds
        self.base_url = "https://api.openai.com/v1"
        self.logger = logging.getLogger(__name__)

        self.model_limits = {
            "gpt-4": {"max_total": 8192, "reserve_output": 1000},
            "gpt-4-32k": {"max_total": 32768, "reserve_output": 2000},
            "gpt-3.5-turbo": {"max_total": 4096, "reserve_output": 500},
            "gpt-3.5-turbo-16k": {"max_total": 16384, "reserve_output": 1000},
            "default": {"max_total": 4096, "reserve_output": 500},
        }

    async def generate(
        self,
        prompt: str,
        model: str,
        temperature: float = 0.7,
        max_tokens: int = 4000,
        top_p: float = 0.9,
        **kwargs,
    ) -> LLMResponse:
        """Generate text using OpenAI API."""
        # TODO: Implement OpenAI API calls
        # This is a stub for future implementation
        return LLMResponse(
            content="",
            success=False,
            error="OpenAI provider not yet implemented",
            processing_time_ms=0,
        )

    async def test_connection(self) -> bool:
        """Test connection to OpenAI API."""
        # TODO: Implement connection test
        return False

    def get_model_limits(self, model: str) -> Dict[str, int]:
        """Get token limits for OpenAI models."""
        if model in self.model_limits:
            return self.model_limits[model]

        if model.startswith("gpt-4"):
            return self.model_limits.get("gpt-4", self.model_limits["default"])
        elif model.startswith("gpt-3.5"):
            return self.model_limits.get("gpt-3.5-turbo", self.model_limits["default"])

        return self.model_limits["default"]


PROVIDER_REGISTRY = {
    "ollama": OllamaProvider,
    "openai": OpenAIProvider,
}


def get_provider_class(provider_name: str) -> type:
    """
    Get provider class by name.

    Args:
        provider_name: Name of the provider

    Returns:
        Provider class

    Raises:
        ValueError: If provider not found
    """
    if provider_name not in PROVIDER_REGISTRY:
        available = ", ".join(PROVIDER_REGISTRY.keys())
        raise ValueError(
            f"Unknown LLM provider: {provider_name}. "
            f"Available providers: {available}"
        )
    return PROVIDER_REGISTRY[provider_name]


def register_provider(name: str, provider_class: type):
    """
    Register a new provider.

    Args:
        name: Provider name
        provider_class: Provider class (must inherit from LLMProvider)
    """
    if not issubclass(provider_class, LLMProvider):
        raise TypeError(
            f"Provider class must inherit from LLMProvider, "
            f"got {provider_class.__name__}"
        )
    PROVIDER_REGISTRY[name] = provider_class
