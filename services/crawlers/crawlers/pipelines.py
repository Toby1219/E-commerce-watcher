# Define your item pipelines here
#
# Don't forget to add your pipeline to the ITEM_PIPELINES setting
# See: https://docs.scrapy.org/en/latest/topics/item-pipeline.html


# useful for handling different item types with a single interface
from itemadapter import ItemAdapter
from tortoise import Tortoise  
from itemadapter import ItemAdapter 
import logging
from .db.models import Product  
from .db.models import Currencies
from .db.config import TORTOISE_ORM

logger = logging.getLogger(__name__)
class TortoiseDBPipeline:
    
    @classmethod
    def from_crawler(cls, crawler):
        pipeline = cls()
        pipeline.crawler = crawler
        return pipeline
    
    async def open_spider(self, spider):
        # Initialize Tortoise ORM connection pool globally
        if not Tortoise._inited:
            await Tortoise.init(config=TORTOISE_ORM, _enable_global_fallback=True)
            logger.info("Tortoise ORM connection pool opened for spider: %s", spider.name)
        else:
            logger.info("Tortoise ORM already initialized, reusing pool for spider: %s", spider.name)

    async def close_spider(self):
        await Tortoise.close_connections()
        logger.info("Tortoise ORM connection pool closed")

    async def process_item(self, item):
        adapter = ItemAdapter(item)
        product_name_ = adapter.get("product_name")
        prod_name = product_name_.replace("+", "").strip() if product_name_ else None
        product, created = await Product.get_or_create(
            product_url=adapter.get("product_url"), 
            defaults={
                "product_name": prod_name,
                "product_price": adapter.get("product_price"),
                "product_price_text": adapter.get("product_price_text"),
                "product_currency": adapter.get("product_currency"),
                "product_brand_name": adapter.get("product_brand"),
                "product_category": adapter.get("product_category"),
                "product_image": adapter.get("product_image"),
                "product_sku": adapter.get("product_sku"),
                "product_details": adapter.get("product_details", {}), 
                "product_reviews": adapter.get("product_reviews", {}),
                "product_specifications": adapter.get("product_specifications", {}),
                "score": adapter.get("score", 0),
            },
        )

        if not created:
            product.product_price = adapter.get("product_price")
            product.product_details = adapter.get("product_details", {})
            product.product_reviews = adapter.get("product_reviews", {})
            product.product_specifications = adapter.get("product_specifications", {})
            product.score = adapter.get("score", 0)
            await product.save()  
            logger.debug(f"Updated existing product: {product.product_url}")
        else:
            logger.debug(f"Inserted new product: {product.product_url}")
        return item
    
    
class CurrencyItemsPipeline:    
    @classmethod
    def from_crawler(cls, crawler):
        pipeline = cls()
        pipeline.crawler = crawler
        return pipeline
    
    async def open_spider(self, spider):
        if not Tortoise._inited:
            await Tortoise.init(config=TORTOISE_ORM, _enable_global_fallback=True)
            logger.info("Tortoise ORM connection pool opened for spider: %s", spider.name)
        else:
            logger.info("Tortoise ORM already initialized, reusing pool for spider: %s", spider.name)
    
    async def close_spider(self):
        await Tortoise.close_connections()
        logger.info("Tortoise ORM connection pool closed")

    async def process_item(self, item):
        adapter = ItemAdapter(item)
        currency, created = await Currencies.update_or_create(
            base_currency=adapter.get("base_currency"),
            currency = adapter.get("currency"),
            defaults={
                "rates":adapter.get("rates"),
                "time_last_update_utc":adapter.get("time_last_update_utc"),
                "time_next_update_utc":adapter.get("time_next_update_utc")
            }
        )
        if not created:
            currency.base_currency = adapter.get("base_currency")
            currency.currency = adapter.get("currency")
            currency.rates = adapter.get("rates")
            currency.time_last_update_utc = adapter.get("time_last_update_utc")
            currency.time_next_update_utc = adapter.get("time_next_update_utc")
            await currency.save()
        return item