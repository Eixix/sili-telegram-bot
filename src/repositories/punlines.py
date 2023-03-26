import json
from enum import Enum
from typing import Dict, List

_punlines: Dict[str, Dict[str, List[str]]] = None


class PunlinesTypes(Enum):
    Dodo_Poll = 0
    Match_Outcome = 1
    Performance_Verbs = 2


def get_punlines(punline_type: PunlinesTypes) -> Dict[str, List[str]]:
    if _punlines is None:
        _initialize_punlines()

    match punline_type:
        case PunlinesTypes.Dodo_Poll:
            return _punlines["dodo_poll"]
        case PunlinesTypes.Match_Outcome:
            return _punlines["match_outcome"]
        case PunlinesTypes.Performance_Verbs:
            return _punlines["performance_verbs"]


def _initialize_punlines() -> None:
    global _punlines
    with open("resources/punlines.json", 'r', encoding="utf8") as f:
        _punlines = json.load(f)
