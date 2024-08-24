"""
Construct a config object from default config file, user config file, and env vars.
Much of the behavior is controlled via  default config file's 'metaconfig' field.
"""

import json
import os
import configparser as cfp

_default_config_file = "./config/default.json"

with open(_default_config_file, "r") as infile:
    _default_config = json.load(infile)

# Config values to be read from env. Keys represent the env var name, are len 2 lists
# where the firt element specifies the section, the other the option name.
_env_vars = _default_config["metaconfig"]["env_variables"]

# Get the user-specific config file.
_user_config_file = _default_config["metaconfig"]["user_config_file"]

# Remove metaconfig before initializing the object, it shouldn't be depended on for
# anything other than this config bit.
del _default_config["metaconfig"]

config = cfp.ConfigParser(allow_no_value=True)

# Set order of precedence, first default, then used, then env.
config.read_dict(_default_config)

if os.path.exists(_user_config_file):
    with open(_user_config_file, "r") as infile:
        config.read_dict(json.load(infile))

for env_name, env_path in _env_vars.items():
    update_value = os.environ.get(env_name)

    # Only update if the env var was set.
    if update_value:
        section = config[env_path[0]]
        section[env_path[1]] = update_value
