import scrapy
import patchright
from scrapy_playwright.page import PageMethod
from playwright.async_api import Page

class Spider002Spider(scrapy.Spider):
    name = "spider002"
    
    custom_settings = {
                
        "DOWNLOAD_HANDLERS" : {
            "https": "scrapy_playwright.handler.ScrapyPlaywrightDownloadHandler",
        },
        "PLAYWRIGHT_BROWSER_PROVIDER": "scrapy_playwright.provider.PlaywrightBrowserProvider",

        "TWISTED_REACTOR" : "twisted.internet.asyncioreactor.AsyncioSelectorReactor",
        "PLAYWRIGHT_BROWSER_TYPE": "chromium",

        "PLAYWRIGHT_LAUNCH_OPTIONS": {
            "headless": False,
            "executable_path": "/usr/bin/google-chrome",
            "timeout": 20 * 1000,
        },

        "PLAYWRIGHT_DEFAULT_NAVIGATION_TIMEOUT": 90 * 1000,

        "USER_AGENT": None,
        
        "PLAYWRIGHT_CONTEXTS": {
            "default": {
                "user_agent": (
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
                    "(KHTML, like Gecko) Chrome/131.0.6778.85 Safari/537.36"
                ),
                "viewport": {"width": 1920, "height": 1080},
                "locale": "en-US",
            },
        },
        "HTTPCACHE_ENABLED": False
    }
    
    def __init__(self, search_key:str = "hp laptop", name = None, **kwargs):
        super().__init__(name, **kwargs)
        self.search_key = search_key
        self.domain = "https://www.ebay.com/"
        
        
    async def start(self):
        url = "https://www.ebay.com/" # f"https://jiji.ng/search?query={self.search_key}"
        yield scrapy.Request(
            url, 
            meta = {
                "playwright": True, 
                "playwright_include_page": True,
                "playwright_page_methods": [
                    # PageMethod("wait_for_selector", "div.masonry-item", timeout=9000),
                    PageMethod("goto", self.domain),
                    PageMethod("wait_for_selector", "input.gh-search-input"),
                    PageMethod("fill", "input.gh-search-input", self.search_key),
                    PageMethod("press", "input.gh-search-input", "Enter"),
                    PageMethod("wait_for_selector", "ul.srp-results > li.s-card")
                ]
            },
            callback=self.parse
        )
        
    async def parse(self, response):
        page:Page = response.meta["playwright_page"]
        b = response.css("ul.srp-results > li.s-card").getall()
        yield{
            "name": response.url,
            "items": len(b)
        }




