import argparse
from scrapy.crawler import AsyncCrawlerProcess
from scrapy.utils.project import get_project_settings
from crawlers.spiders.spider001 import Spider001Spider

def main(search:str="itel"):
    setting = get_project_settings()
    process = AsyncCrawlerProcess(setting)
    process.crawl(Spider001Spider, search_key=search)
    process.start()

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--search", required=True)
    args = parser.parse_args()
    main(args.search)
    # main()