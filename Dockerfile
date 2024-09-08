FROM python:3.10
WORKDIR /bot

ARG bot_token
ARG chat_id

ENV bot_token=${bot_token}
ENV chat_id=${chat_id}

COPY pyproject.toml pyproject.toml
COPY config config
COPY src src
COPY resources resources
COPY config.json config.json

VOLUME [ "/bot/resources/dynamic/matchdata" ]
RUN pip install .

WORKDIR /bot
ENTRYPOINT [ "run_bot" ]

