import logging, dota_api, time
import telegram_send

logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
                     level=logging.INFO)

while True:
    messages = dota_api.api_crawl()

    for message in messages:
        print(message)
        telegram_send.send(messages=[message])
    time.sleep(600)