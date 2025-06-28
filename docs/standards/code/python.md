# Python Standards

This document outlines the specific coding standards for Python development.

## Overview

We follow Python's official style guide (PEP 8) with additional project-specific conventions. All Python code should be Python 3.10+ compatible.

## Python Version

- **Minimum Version**: Python 3.10
- **Recommended Version**: Latest stable Python 3.x
- **Version Management**: Use `pyenv` for version management
- **Virtual Environments**: Always use virtual environments (`venv` or `poetry`)

## Code Formatting

### Black Configuration

```toml
# pyproject.toml
[tool.black]
line-length = 88
target-version = ['py310']
include = '\.pyi?$'
extend-exclude = '''
/(
  # directories
  \.eggs
  | \.git
  | \.hg
  | \.mypy_cache
  | \.tox
  | \.venv
  | _build
  | buck-out
  | build
  | dist
)/
'''
```

### Style Rules

```python
# Use 4 spaces for indentation (never tabs)
def calculate_total(items):
    total = 0
    for item in items:
        total += item.price
    return total

# Maximum line length of 88 characters (Black default)
def very_long_function_name_that_might_exceed_line_length(
    parameter_one, parameter_two, parameter_three
):
    return parameter_one + parameter_two + parameter_three

# Use trailing commas in multi-line structures
ALLOWED_EXTENSIONS = [
    '.jpg',
    '.jpeg',
    '.png',
    '.gif',
]

# Use double quotes for strings (Black will enforce consistency)
message = "Hello, world!"
multiline_message = """
This is a multiline string
that spans multiple lines.
"""
```

## Naming Conventions

### Variables and Functions

```python
# snake_case for variables and functions
user_name = "john_doe"
total_amount = 100.50

def get_user_by_id(user_id):
    """Retrieve user by their unique identifier."""
    return database.users.find_one({"_id": user_id})

def calculate_monthly_revenue():
    """Calculate total revenue for the current month."""
    # implementation
    pass

# Boolean variables should be descriptive
is_authenticated = True
has_permission = False
can_edit = user.role == "admin"
```

### Classes and Constants

```python
# PascalCase for classes
class UserService:
    """Service for managing user operations."""

    def __init__(self, database_client):
        self.db = database_client

    def create_user(self, user_data):
        """Create a new user with the provided data."""
        # implementation
        pass

# UPPER_SNAKE_CASE for constants
MAX_RETRY_ATTEMPTS = 3
DEFAULT_TIMEOUT_SECONDS = 30
API_BASE_URL = "https://api.example.com"

# Module-level constants
DATABASE_URL = os.getenv("DATABASE_URL")
SECRET_KEY = os.getenv("SECRET_KEY")
```

### Private Methods and Variables

```python
class BankAccount:
    def __init__(self, initial_balance):
        self.balance = initial_balance
        self._account_number = self._generate_account_number()  # Protected
        self.__pin = self._generate_pin()  # Private

    def _generate_account_number(self):
        """Protected method for internal use."""
        # implementation
        pass

    def __generate_pin(self):
        """Private method - name mangled."""
        # implementation
        pass
```

## Type Hints

### Basic Type Hints

```python
from typing import List, Dict, Optional, Union, Callable, Any
from datetime import datetime

# Function with type hints
def process_user_data(
    user_id: str,
    data: Dict[str, Any],
    callback: Optional[Callable[[str], None]] = None
) -> bool:
    """Process user data and optionally call callback function."""
    # implementation
    if callback:
        callback(user_id)
    return True

# Class with type hints
class User:
    def __init__(self, name: str, age: int, email: Optional[str] = None):
        self.name = name
        self.age = age
        self.email = email
        self.created_at: datetime = datetime.now()

    def is_adult(self) -> bool:
        return self.age >= 18
```

### Advanced Type Hints

```python
from typing import TypeVar, Generic, Protocol, Union, Literal
from dataclasses import dataclass

# Generic types
T = TypeVar('T')

class Repository(Generic[T]):
    def save(self, entity: T) -> T:
        # implementation
        pass

    def find_by_id(self, entity_id: str) -> Optional[T]:
        # implementation
        pass

# Protocol for structural typing
class Drawable(Protocol):
    def draw(self) -> None: ...

# Literal types for specific values
Status = Literal["pending", "approved", "rejected"]

def update_status(user_id: str, status: Status) -> None:
    # implementation
    pass

# Union types
def process_input(data: Union[str, int, List[str]]) -> str:
    if isinstance(data, str):
        return data.upper()
    elif isinstance(data, int):
        return str(data)
    else:
        return ", ".join(data)
```

### Dataclasses and Pydantic Models

```python
from dataclasses import dataclass, field
from typing import List, Optional
from datetime import datetime
from pydantic import BaseModel, validator

# Dataclass for simple data structures
@dataclass
class UserProfile:
    name: str
    email: str
    age: int
    tags: List[str] = field(default_factory=list)
    created_at: datetime = field(default_factory=datetime.now)

    def is_adult(self) -> bool:
        return self.age >= 18

# Pydantic models for API and validation
class CreateUserRequest(BaseModel):
    name: str
    email: str
    age: int

    @validator('email')
    def email_must_be_valid(cls, v):
        if '@' not in v:
            raise ValueError('Invalid email address')
        return v

    @validator('age')
    def age_must_be_positive(cls, v):
        if v < 0:
            raise ValueError('Age must be positive')
        return v

class UserResponse(BaseModel):
    id: str
    name: str
    email: str
    age: int
    is_active: bool
    created_at: datetime

    class Config:
        orm_mode = True  # For SQLAlchemy integration
```

## Error Handling

### Exception Classes

```python
# Create specific exception classes
class ValidationError(Exception):
    """Raised when data validation fails."""

    def __init__(self, message: str, field: str, value: Any):
        super().__init__(message)
        self.field = field
        self.value = value

class UserNotFoundError(Exception):
    """Raised when a user cannot be found."""

    def __init__(self, user_id: str):
        super().__init__(f"User with ID {user_id} not found")
        self.user_id = user_id

class APIError(Exception):
    """Raised when an API call fails."""

    def __init__(self, message: str, status_code: int, response_data: Optional[Dict] = None):
        super().__init__(message)
        self.status_code = status_code
        self.response_data = response_data or {}
```

### Error Handling Patterns

```python
import logging
from typing import Result, Union

# Set up logging
logger = logging.getLogger(__name__)

# Result pattern for operations that can fail
class Result:
    def __init__(self, success: bool, data: Any = None, error: Optional[Exception] = None):
        self.success = success
        self.data = data
        self.error = error

    @classmethod
    def ok(cls, data: Any) -> "Result":
        return cls(success=True, data=data)

    @classmethod
    def error(cls, error: Exception) -> "Result":
        return cls(success=False, error=error)

def fetch_user(user_id: str) -> Result:
    """Fetch user by ID, returning a Result object."""
    try:
        user = database.users.find_one({"_id": user_id})
        if not user:
            return Result.error(UserNotFoundError(user_id))
        return Result.ok(user)
    except Exception as e:
        logger.error(f"Failed to fetch user {user_id}: {e}")
        return Result.error(e)

# Using the Result pattern
result = fetch_user("123")
if result.success:
    user = result.data
    print(f"Found user: {user['name']}")
else:
    print(f"Error: {result.error}")
```

### Context Managers

```python
from contextlib import contextmanager
import sqlite3

@contextmanager
def database_transaction():
    """Context manager for database transactions."""
    conn = sqlite3.connect("database.db")
    try:
        yield conn
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()

# Usage
try:
    with database_transaction() as conn:
        cursor = conn.cursor()
        cursor.execute("INSERT INTO users (name, email) VALUES (?, ?)",
                      ("John Doe", "john@example.com"))
except Exception as e:
    logger.error(f"Database transaction failed: {e}")
```

## Import Organization

### Import Structure

```python
# 1. Standard library imports
import os
import sys
import json
import logging
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Dict, Optional, Any

# 2. Third-party imports
import requests
import pandas as pd
from fastapi import FastAPI, HTTPException
from sqlalchemy import create_engine
from pydantic import BaseModel

# 3. Local application imports
from .models import User, UserProfile
from .services.user_service import UserService
from .utils.validation import validate_email
from ..config import settings
```

### Import Patterns

```python
# Prefer specific imports over wildcard imports
from datetime import datetime, timedelta  # Good
from datetime import *  # Bad

# Use aliases for long module names
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

# Import types separately for clarity
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .models import User  # Avoid circular imports
```

## Async/Await

### Async Function Patterns

```python
import asyncio
import aiohttp
from typing import List

async def fetch_user_data(session: aiohttp.ClientSession, user_id: str) -> Dict:
    """Fetch user data asynchronously."""
    async with session.get(f"/users/{user_id}") as response:
        response.raise_for_status()
        return await response.json()

async def fetch_multiple_users(user_ids: List[str]) -> List[Dict]:
    """Fetch multiple users concurrently."""
    async with aiohttp.ClientSession() as session:
        tasks = [fetch_user_data(session, user_id) for user_id in user_ids]
        return await asyncio.gather(*tasks)

# Error handling with async
async def safe_fetch_user(session: aiohttp.ClientSession, user_id: str) -> Optional[Dict]:
    """Safely fetch user data with error handling."""
    try:
        return await fetch_user_data(session, user_id)
    except aiohttp.ClientError as e:
        logger.error(f"Failed to fetch user {user_id}: {e}")
        return None
```

### Async Context Managers

```python
from contextlib import asynccontextmanager
import aiofiles

@asynccontextmanager
async def async_file_handler(filepath: str, mode: str = "r"):
    """Async context manager for file handling."""
    file = await aiofiles.open(filepath, mode)
    try:
        yield file
    finally:
        await file.close()

# Usage
async def process_file(filepath: str) -> str:
    async with async_file_handler(filepath) as file:
        content = await file.read()
        return content.upper()
```

## Database Integration

### SQLAlchemy Models

```python
from sqlalchemy import Column, Integer, String, Boolean, DateTime, ForeignKey
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import relationship, Session
from datetime import datetime

Base = declarative_base()

class User(Base):
    __tablename__ = "users"

    id = Column(Integer, primary_key=True, index=True)
    email = Column(String(255), unique=True, index=True, nullable=False)
    name = Column(String(255), nullable=False)
    is_active = Column(Boolean, default=True)
    created_at = Column(DateTime, default=datetime.utcnow)

    # Relationships
    profile = relationship("UserProfile", back_populates="user", uselist=False)
    posts = relationship("Post", back_populates="author")

class UserProfile(Base):
    __tablename__ = "user_profiles"

    id = Column(Integer, primary_key=True, index=True)
    user_id = Column(Integer, ForeignKey("users.id"), nullable=False)
    bio = Column(String(1000))
    avatar_url = Column(String(500))

    # Relationships
    user = relationship("User", back_populates="profile")
```

### Repository Pattern

```python
from abc import ABC, abstractmethod
from typing import List, Optional
from sqlalchemy.orm import Session

class UserRepository(ABC):
    """Abstract repository for user operations."""

    @abstractmethod
    def find_by_id(self, user_id: int) -> Optional[User]:
        pass

    @abstractmethod
    def find_by_email(self, email: str) -> Optional[User]:
        pass

    @abstractmethod
    def save(self, user: User) -> User:
        pass

    @abstractmethod
    def delete(self, user_id: int) -> bool:
        pass

class SQLAlchemyUserRepository(UserRepository):
    """SQLAlchemy implementation of user repository."""

    def __init__(self, db_session: Session):
        self.db = db_session

    def find_by_id(self, user_id: int) -> Optional[User]:
        return self.db.query(User).filter(User.id == user_id).first()

    def find_by_email(self, email: str) -> Optional[User]:
        return self.db.query(User).filter(User.email == email).first()

    def save(self, user: User) -> User:
        self.db.add(user)
        self.db.commit()
        self.db.refresh(user)
        return user

    def delete(self, user_id: int) -> bool:
        user = self.find_by_id(user_id)
        if user:
            self.db.delete(user)
            self.db.commit()
            return True
        return False
```

## Testing Standards

### Test Structure

```python
import pytest
from unittest.mock import Mock, patch, AsyncMock
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from fastapi.testclient import TestClient

from ..main import app
from ..models import User, Base
from ..services.user_service import UserService

# Test fixtures
@pytest.fixture
def test_db():
    """Create test database."""
    engine = create_engine("sqlite:///:memory:")
    Base.metadata.create_all(engine)
    TestingSessionLocal = sessionmaker(bind=engine)
    return TestingSessionLocal()

@pytest.fixture
def sample_user():
    """Provide sample user data."""
    return {
        "email": "test@example.com",
        "name": "Test User",
        "age": 30
    }

@pytest.fixture
def mock_user_repository():
    """Mock user repository."""
    return Mock(spec=UserRepository)

# Test classes
class TestUserService:
    """Test cases for UserService."""

    def setup_method(self):
        """Set up test method."""
        self.mock_repo = Mock(spec=UserRepository)
        self.user_service = UserService(self.mock_repo)

    def test_create_user_success(self, sample_user):
        """Test successful user creation."""
        # Arrange
        expected_user = User(id=1, **sample_user)
        self.mock_repo.save.return_value = expected_user

        # Act
        result = self.user_service.create_user(sample_user)

        # Assert
        assert result.email == sample_user["email"]
        assert result.name == sample_user["name"]
        self.mock_repo.save.assert_called_once()

    def test_create_user_duplicate_email(self, sample_user):
        """Test user creation with duplicate email."""
        # Arrange
        self.mock_repo.find_by_email.return_value = User(id=1, **sample_user)

        # Act & Assert
        with pytest.raises(ValidationError, match="Email already exists"):
            self.user_service.create_user(sample_user)

    @pytest.mark.parametrize("age,expected", [
        (17, False),
        (18, True),
        (65, True),
        (100, True)
    ])
    def test_is_adult(self, age, expected):
        """Test age validation with multiple scenarios."""
        user = User(name="Test", email="test@example.com", age=age)
        assert user.is_adult() == expected

# Async test examples
class TestAsyncUserService:
    """Test cases for async user service."""

    @pytest.mark.asyncio
    async def test_fetch_user_data(self):
        """Test async user data fetching."""
        # Arrange
        mock_session = AsyncMock()
        mock_response = AsyncMock()
        mock_response.json.return_value = {"id": 1, "name": "Test User"}
        mock_session.get.return_value.__aenter__.return_value = mock_response

        # Act
        result = await fetch_user_data(mock_session, "1")

        # Assert
        assert result["id"] == 1
        assert result["name"] == "Test User"

# Integration tests
class TestUserAPI:
    """Integration tests for user API."""

    def setup_method(self):
        """Set up test method."""
        self.client = TestClient(app)

    def test_create_user_endpoint(self, sample_user):
        """Test user creation endpoint."""
        response = self.client.post("/users", json=sample_user)

        assert response.status_code == 201
        data = response.json()
        assert data["email"] == sample_user["email"]
        assert "id" in data
```

### Mock Patterns

```python
from unittest.mock import Mock, patch, MagicMock

# Mock external services
@patch('requests.get')
def test_external_api_call(mock_get):
    """Test external API call with mocking."""
    # Arrange
    mock_response = Mock()
    mock_response.json.return_value = {"status": "success"}
    mock_response.status_code = 200
    mock_get.return_value = mock_response

    # Act
    result = call_external_api()

    # Assert
    assert result["status"] == "success"
    mock_get.assert_called_once_with("https://api.example.com/data")

# Mock database operations
def test_user_service_with_mock_db(mock_user_repository):
    """Test user service with mocked database."""
    # Arrange
    user_service = UserService(mock_user_repository)
    user_data = {"email": "test@example.com", "name": "Test User"}

    # Act
    user_service.create_user(user_data)

    # Assert
    mock_user_repository.save.assert_called_once()
```

## Configuration and Settings

### Environment Variables

```python
import os
from typing import Optional
from pydantic import BaseSettings, validator

class Settings(BaseSettings):
    """Application settings."""

    # Database
    database_url: str
    database_pool_size: int = 10

    # API
    api_key: str
    api_timeout: int = 30

    # Security
    secret_key: str
    jwt_expiration_hours: int = 24

    # Feature flags
    enable_debug: bool = False
    enable_cors: bool = True

    @validator('database_url')
    def database_url_must_be_valid(cls, v):
        if not v.startswith(('postgresql://', 'sqlite://')):
            raise ValueError('Invalid database URL')
        return v

    class Config:
        env_file = ".env"
        case_sensitive = False

# Usage
settings = Settings()
```

### Logging Configuration

```python
import logging
import sys
from pathlib import Path

def setup_logging(log_level: str = "INFO", log_file: Optional[str] = None):
    """Set up application logging."""

    # Create formatter
    formatter = logging.Formatter(
        fmt="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )

    # Set up console handler
    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setFormatter(formatter)

    # Set up file handler if specified
    handlers = [console_handler]
    if log_file:
        file_handler = logging.FileHandler(log_file)
        file_handler.setFormatter(formatter)
        handlers.append(file_handler)

    # Configure root logger
    logging.basicConfig(
        level=getattr(logging, log_level.upper()),
        handlers=handlers
    )

    # Configure specific loggers
    logging.getLogger("sqlalchemy.engine").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)

# Usage
setup_logging(log_level="DEBUG", log_file="app.log")
logger = logging.getLogger(__name__)
```

## Configuration Files

### Setup Configuration (`setup.cfg`)

```ini
[metadata]
name = my-project
version = 1.0.0
description = Project description
author = Your Name
author_email = your.email@example.com

[options]
packages = find:
python_requires = >=3.10
install_requires =
    fastapi>=0.68.0
    uvicorn[standard]>=0.15.0
    sqlalchemy>=1.4.0
    pydantic>=1.8.0

[options.extras_require]
dev =
    pytest>=6.2.0
    pytest-cov>=2.12.0
    black>=21.0.0
    flake8>=3.9.0
    mypy>=0.910

[flake8]
max-line-length = 88
extend-ignore = E203, W503
exclude = .git,__pycache__,build,dist
per-file-ignores =
    __init__.py:F401

[mypy]
python_version = 3.10
warn_return_any = True
warn_unused_configs = True
disallow_untyped_defs = True
disallow_incomplete_defs = True
check_untyped_defs = True
disallow_untyped_decorators = True
no_implicit_optional = True
warn_redundant_casts = True
warn_unused_ignores = True
warn_no_return = True
warn_unreachable = True
strict_equality = True

[coverage:run]
source = src
omit =
    tests/*
    */migrations/*
    */venv/*

[coverage:report]
precision = 2
show_missing = True
skip_covered = False
exclude_lines =
    pragma: no cover
    def __repr__
    raise AssertionError
    raise NotImplementedError
```

### pyproject.toml

```toml
[build-system]
requires = ["setuptools>=45", "wheel", "setuptools_scm>=6.2"]
build-backend = "setuptools.build_meta"

[project]
name = "my-project"
version = "1.0.0"
description = "Project description"
authors = [{name = "Your Name", email = "your.email@example.com"}]
requires-python = ">=3.10"
dependencies = [
    "fastapi>=0.68.0",
    "uvicorn[standard]>=0.15.0",
    "sqlalchemy>=1.4.0",
    "pydantic>=1.8.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=6.2.0",
    "pytest-cov>=2.12.0",
    "black>=21.0.0",
    "flake8>=3.9.0",
    "mypy>=0.910",
]

[tool.black]
line-length = 88
target-version = ['py310']
include = '\.pyi?

[tool.isort]
profile = "black"
line_length = 88
multi_line_output = 3

[tool.pytest.ini_options]
minversion = "6.0"
addopts = "-ra -q --strict-markers --strict-config"
testpaths = ["tests"]
pythonpath = ["src"]
```

---

For more general coding standards that apply to all languages, see [our standards guide.](README.md).
