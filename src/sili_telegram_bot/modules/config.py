"""
Construct a config object from default config file, user config file, and env vars.
Much of the behavior is controlled via  default config file's 'metaconfig' field.
"""

import json
import logging
import os
import configparser as cfp

from ast import literal_eval

_default_config_file = "./config/default.json"


def _init_config(default_config_file) -> cfp.ConfigParser:
    """
    Initialize the CONFIG object by reading the default config, then the user config,
    and then updating it with pre-defined variables from the environment.
    """
    with open(default_config_file, "r") as infile:
        default_config = json.load(infile)

    # Config values to be read from env. Keys represent the env var name, are len 2 lists
    # where the firt element specifies the section, the other the option name.
    env_vars = default_config["metaconfig"]["env_variables"]

    # Get the user-specific config file.
    user_config_file = default_config["metaconfig"]["user_config_file"]

    # Remove metaconfig before initializing the object, it shouldn't be depended on for
    # anything other than this config bit.
    del default_config["metaconfig"]

    config = cfp.ConfigParser(allow_no_value=True)

    # Set order of precedence, first default, then used, then env.
    config.read_dict(default_config)

    if os.path.exists(user_config_file):
        with open(user_config_file, "r") as infile:
            config.read_dict(json.load(infile))

    for env_name, env_path in env_vars.items():
        update_value = os.environ.get(env_name)

        # Only update if the env var was set.
        if update_value:
            section = config[env_path[0]]
            section[env_path[1]] = update_value

    return config


config = _init_config(_default_config_file)


def get_accounts() -> list[dict]:
    """
    De-serialize the string representation of the accounts list.
    """
    # FIXME Use some other config library that allows for deeper nesting.
    return literal_eval(config["accounts"]["account_list"])


def _config_logging(level: str = "info") -> None:
    """
    Perform configuration for the logging module.
    """

    match level.lower():
        case "critical":
            log_level = logging.CRITICAL
        case "error":
            log_level = logging.ERROR
        case "warning":
            log_level = logging.WARNING
        case "info":
            log_level = logging.INFO
        case "debug":
            log_level = logging.DEBUG
        case _:
            raise ValueError(f"Unknown log level: {level}.")

    logging.basicConfig(
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        level=log_level,
    )


_config_logging(level=config["logging"]["level"])
