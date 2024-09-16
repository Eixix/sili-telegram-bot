"""
Type definitions related to responses & voicelines.
"""

from typing import TypedDict


class EntityResponse(TypedDict):
    text: str
    urls: list[str, None]


class EntityData(TypedDict):
    name: str
    title: str
    url: str
