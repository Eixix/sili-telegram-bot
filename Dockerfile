FROM python:3.10
WORKDIR /bot

ARG bot_token
ARG chat_id

ENV bot_token=${bot_token}
ENV chat_id=${chat_id}

COPY pyproject.toml pyproject.toml
COPY config config
COPY src /bot/src
COPY resources resources
COPY matchdata/accounts_file.json matchdata/accounts_file.json

VOLUME [ "/bot/matchdata" ]
RUN pip install .

WORKDIR /bot
ENTRYPOINT [ "run_bot" ]

