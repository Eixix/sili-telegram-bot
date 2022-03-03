FROM python:3.10
WORKDIR /bot

ARG bot_token
ARG chat_id

ENV bot_token=${bot_token}
ENV chat_id=${chat_id}

COPY src /bot/src
COPY requirements.txt requirements.txt
COPY heroes.json heroes.json
COPY punlines.json punlines.json
COPY matchdata/accounts_file.json matchdata/accounts_file.json

VOLUME [ "/app/matchdata" ]
RUN pip install -r requirements.txt

WORKDIR /bot/src
ENTRYPOINT [ "./bot.py" ]

