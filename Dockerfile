FROM python:3.10
WORKDIR /bot

ARG bot_token
ARG chat_id

ENV bot_token=${bot_token}
ENV chat_id=${chat_id}

COPY src /bot/src
COPY requirements.txt requirements.txt
COPY resources resources
COPY matchdata/accounts_file.json matchdata/accounts_file.json

VOLUME [ "/app/matchdata" ]
RUN pip install .

WORKDIR /bot
ENTRYPOINT [ "src/bot.py" ]

