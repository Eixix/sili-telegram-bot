FROM python:3.10
WORKDIR /bot

ARG bot_token
ARG chat_id

ENV bot_token=${bot_token}
ENV chat_id=${chat_id}

COPY config config
COPY src src
COPY resources resources

# config.json may or may not be present, this ensures it is copied when it's
# there and doesn't result in errors when it's not.
COPY pyproject.toml config.json* ./

RUN pip install .

WORKDIR /bot
ENTRYPOINT [ "run_bot" ]

