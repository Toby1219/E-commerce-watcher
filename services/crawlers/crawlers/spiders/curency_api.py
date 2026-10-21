import scrapy
from decimal import Decimal
from email.utils import parsedate_to_datetime

class CurencyApiSpider(scrapy.Spider):
    name = "curency_api"

    custom_settings = {
        "ITEM_PIPELINES": {
            "crawlers.pipelines.CurrencyItemsPipeline": 300,
        }
    }
    
    async def start(self):
        url = "https://v6.exchangerate-api.com/v6/d413960e4a51d7258f6c4d81/latest/USD"
        yield scrapy.Request(url=url, callback=self.parse)
    
    async def parse(self, response):
        body = response.json()
        base_currency = body["base_code"]
        time_last_update_utc = body["time_last_update_utc"]
        time_next_update_utc = body["time_next_update_utc"]
        
        curencies = body["conversion_rates"]
        for k, v in curencies.items():
            currency = k
            rates = v
        
            yield{
                "base_currency": base_currency,
                "currency":currency,
                "rates":Decimal(str(rates)),
                "time_last_update_utc":parsedate_to_datetime(time_last_update_utc),
                "time_next_update_utc":parsedate_to_datetime(time_next_update_utc)
            }
