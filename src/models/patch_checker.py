import hashlib
import logging
import requests


class PatchChecker:
    logger = logging.getLogger(__name__)

    URL = "https://www.dota2.com/patches/7.33"
    HEADERS = {'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_10_1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.95 Safari/537.36',
               'Pragma': 'no-cache', 'Cache-Control': 'no-cache'}
    current_hash = ""

    def __init__(self) -> None:
        pass

    def get_if_new_patch(self) -> bool:
        self.logger.info("Checking for DOTA2 updates...")
        response = requests.get(self.URL, self.HEADERS)
        newHash = hashlib.sha224(response.text.encode()).hexdigest()

        self.logger.info(
            f"Reading website with hash {newHash}, old hash is {self.current_hash}")

        # Ask if current hash is empty to prevent unnecessary messages
        if newHash != self.current_hash and self.current_hash != "":
            self.current_hash = newHash
            self.logger.info(f"Update found with hash {newHash}")
            self.current_hash = newHash
            return True

        self.current_hash = newHash
        self.logger.info(f"No Update found")
        return False
