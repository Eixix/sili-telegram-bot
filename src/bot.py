#!/usr/bin/env python3

import logging

from src.telegram_service.telegram_functions import initialize_telegram_bot

logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
                    level=logging.INFO)
logger = logging.getLogger(__name__)


def main() -> None:
    initialize_telegram_bot()


if __name__ == '__main__':
    main()
