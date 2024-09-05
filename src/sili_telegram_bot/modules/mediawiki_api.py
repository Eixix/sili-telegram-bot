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

# FIXME Handle connection errors & don't intaniate this on module load already.
mediawiki_api = mediawiki.MediaWiki(
    url=_api_url,
    rate_limit=True,
    rate_limit_wait=_api_rate_limit_wait,
    user_agent=_user_agent,
)
