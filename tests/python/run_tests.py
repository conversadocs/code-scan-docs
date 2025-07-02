#!/usr/bin/env python3
"""
Test runner script for CSD Python plugins.

This script provides a convenient way to run the Python plugin tests
with proper environment setup. It's particularly useful for external
plugin developers who want to validate their plugins against the same
standards as the built-in plugins.

Usage:
    python tests/python/run_tests.py                    # Run all tests
    python tests/python/run_tests.py base              # Run only base analyzer tests
    python tests/python/run_tests.py --coverage        # Run with coverage
    python tests/python/run_tests.py --fast            # Skip slow tests
"""

import sys
import subprocess
from pathlib import Path
import argparse


def main():
    parser = argparse.ArgumentParser(description="Run CSD Python plugin tests")
    parser.add_argument(
        "test_filter", nargs="?", help="Test filter (e.g., 'base', 'python', 'rust')"
    )
    parser.add_argument(
        "--coverage", action="store_true", help="Run tests with coverage reporting"
    )
    parser.add_argument(
        "--fast", action="store_true", help="Skip slow tests (exclude -m slow)"
    )
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    parser.add_argument(
        "--debug", action="store_true", help="Run in debug mode (with pdb on failures)"
    )

    args = parser.parse_args()

    # Get the project root (assuming this script is in tests/python/)
    script_path = Path(__file__).parent
    project_root = script_path.parent.parent

    # Change to project root for consistent paths
    import os

    os.chdir(project_root)

    # Build pytest command
    cmd = ["python", "-m", "pytest"]

    # Add test path
    if args.test_filter:
        if args.test_filter == "base":
            cmd.append("tests/python/test_base_analyzer.py")
        elif args.test_filter == "python":
            cmd.append("tests/python/test_python_analyzer.py")
        elif args.test_filter == "rust":
            cmd.append("tests/python/test_rust_analyzer.py")
        else:
            cmd.extend(["tests/python/", "-k", args.test_filter])
    else:
        cmd.append("tests/python/")

    # Add options
    if args.verbose:
        cmd.append("-v")

    if args.coverage:
        cmd.extend(
            ["--cov=plugins", "--cov-report=term-missing", "--cov-report=html:htmlcov"]
        )

    if args.fast:
        cmd.extend(["-m", "not slow"])

    if args.debug:
        cmd.append("--pdb")

    # Always add some basic options
    cmd.extend(["--tb=short", "--strict-markers"])

    print(f"Running: {' '.join(cmd)}")
    print(f"Working directory: {project_root}")
    print("-" * 50)

    # Check if pytest is available
    try:
        subprocess.run(
            ["python", "-m", "pytest", "--version"], check=True, capture_output=True
        )
    except subprocess.CalledProcessError:
        print("❌ Error: pytest not found. Please install test dependencies:")
        print("   pip install -r tests/python/requirements.txt")
        return 1

    # Run the tests
    try:
        result = subprocess.run(cmd, check=False)
        return result.returncode
    except KeyboardInterrupt:
        print("\n⏹️  Tests interrupted by user")
        return 130
    except Exception as e:
        print(f"❌ Error running tests: {e}")
        return 1


if __name__ == "__main__":
    exit_code = main()

    if exit_code == 0:
        print("\n✅ All tests passed!")
    else:
        print(f"\n❌ Tests failed with exit code {exit_code}")

    sys.exit(exit_code)
