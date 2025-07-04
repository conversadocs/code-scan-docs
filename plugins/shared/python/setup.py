# plugins/shared/python/setup.py
from setuptools import setup, find_packages

setup(
    name="csd-plugin-sdk",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        # Core dependencies for plugin development
    ],
    python_requires=">=3.8",
)
