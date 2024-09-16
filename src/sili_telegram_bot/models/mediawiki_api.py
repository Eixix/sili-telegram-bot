"""
Container module for the project wide mediawiki object. There should be only one to
ensure rate-limiting is not exceeded.
"""

import mediawiki
import datetime

from sili_telegram_bot.modules.config import config

vl_config = config["voicelines"]


_user_agent = (
    f"{vl_config['user_agent_title']} ({vl_config['user_agent_url']}; "
    f"{vl_config['user_agent_email']})"
)

_api_url = vl_config["base_url"] + "/api.php"
_api_rate_limit_wait = datetime.timedelta(
    seconds=int(vl_config["secs_between_requests"])
)


class APIWrapper:
    """
    Singleton wrapper for the mediawiki API.
    """

    _mediawiki_api = None

    @classmethod
    def get_or_create_mediawiki_api(cls) -> mediawiki.MediaWiki:
        if cls._mediawiki_api is None:
            cls._mediawiki_api = mediawiki.MediaWiki(
                url=_api_url,
                rate_limit=True,
                rate_limit_wait=_api_rate_limit_wait,
                user_agent=_user_agent,
            )

        return cls._mediawiki_api
