import logging
import requests


class PatchChecker:
    logger = logging.getLogger(__name__)

    URL = "https://www.dota2.com/datafeed/patchnoteslist?language=english"
    HEADERS = {'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_10_1) '
                             'AppleWebKit/537.36 (KHTML, like Gecko) '
                             'Chrome/39.0.2171.95 Safari/537.36',
               'Pragma': 'no-cache', 'Cache-Control': 'no-cache'}
    known_patches = []

    def __init__(self) -> None:
        pass

    def get_if_new_patch(self) -> tuple[bool, int]:
        self.logger.info("Checking for DOTA2 updates...")
        website_patches = requests.get(
            self.URL, self.HEADERS).json()["patches"]

        # If emtpy bot just started, so it needs to fill up the patches
        if len(self.known_patches) < len(website_patches) and self.known_patches != []:
            new_patch = website_patches[-1]["patch_name"]
            self.logger.info(
                f"New patch found: {new_patch}")
            self.known_patches = website_patches
            return (True, new_patch)
        else:
            self.known_patches = website_patches
            self.logger.info(
                f"No new patches, latest found patch is {self.known_patches[-1]['patch_name']}")
            return (False, "")
