#!/usr/bin/env python3

import dota_api, time
import telegram_send

while True:
    messages = dota_api.api_crawl()
    messages = '\n\n'.join(messages)

    print(messages)

    telegram_send.send(messages=[messages])
    time.sleep(600)