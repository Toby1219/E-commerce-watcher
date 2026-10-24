import scrapy
import re
from decimal import Decimal

class Spider001Spider(scrapy.Spider):
    name = "crawl001"
    
    custom_settings = {
        "ITEM_PIPELINES": {
            "crawlers.pipelines.TortoiseDBPipeline": 300,
        }
    }
    
    def __init__(self, search_key:str, name = None, **kwargs):
        super().__init__(name, **kwargs)
        self.search_key = search_key
        self.domain = "https://www.jumia.com.ng/"
            
    def build_search_pattern(self, user_input: str) -> re.Pattern:
        words = user_input.strip().split()
        escaped_words = [re.escape(word) for word in words]
        lookaheads = [rf'(?=.*\b{w}\b)' for w in escaped_words]
        pattern = re.compile(''.join(lookaheads), re.IGNORECASE)
        return pattern
    
    def price_parser(self, raw_txt:str) -> Decimal | None:
        if not raw_txt:
            return None
        
        base_price_str = raw_txt.split("-")[0]
        numeric_str = re.sub(r'[^\d.]', '', base_price_str)
        
        if not numeric_str:
            return None
        
        return Decimal(numeric_str)
    
    async def start(self):
        for x in range(1, 5):
            url = f"https://www.jumia.com.ng/catalog/?q={self.search_key}&page={x}#catalog-listing"
            yield scrapy.Request(url, callback=self.parse)
            
    async def parse(self, response):
        prods = response.xpath("//article[@class='prd _fb col c-prd']")
        c = 0
        patterns = self.build_search_pattern(self.search_key)
        for prod in prods:
            name =  prod.css("div.info > h3.name::text").get()
            if patterns.search(name):    
                prod_link = prod.css("a.core::attr(href)").get()
                prod_name = name
                prod_sku = prod.css("a.core::attr(data-gtm-id)").get()
                prod_brand = prod.css("a.core::attr(data-gtm-brand)").get()
                prod_category = prod.css("a.core::attr(data-gtm-category)").get()
                prod_image = response.css("div.img-c > img.img::attr(data-src)").getall()[c]
                c += 1
                yield scrapy.Request(url=f"{self.domain}{prod_link}", meta={
                    "url":f"{self.domain}{prod_link}",
                    "name":prod_name,
                    "sku":prod_sku,
                    "brand":prod_brand,
                    "category":prod_category,
                    "image":prod_image
                }, callback=self.parse_prod)
            else:
                continue

    async def parse_prod(self, response):
        price = response.xpath("//span[@class='-b -ubpt -tal -fs24 -prxs']/text()").get(default="").strip()
        
        d_price = self.price_parser(price)

        prod_details = []
        prod_features = []
        prod_specs = []
        
        summary_text = " ".join(response.xpath("//div[@class='markup -mhxl -pvxxl -oxa -sc']/p").getall())
        words = ["<p>", "</p>", "<strong>", "</strong>", "<br>", "<i>", "</i>", "<span>", "</span>", "&amp;", "U+00a0"]
        pattern = "|".join(map(re.escape, words))   
        summary = re.sub(pattern, "", summary_text.strip() if summary_text else "")
        
        details = response.xpath("//div[@class='markup -mhxl -pvxxl -oxa -sc']/table/tbody/tr")
        
        for detail in details:
            k = detail.css("td::text").get()
            v = detail.css("td:nth-child(2)::text").get()
            prod_details.append({"data":{k:v}})
        
        prod_detail = {"summary": summary, "details":prod_details}
        
        head_features = response.css("article.-pvm:nth-child(1) > div > h3::text").get()
        head_specs = response.css("article.-pvm:nth-child(2) > div > h3::text").get()
        
        items_features = response.css("article.-pvm:nth-child(1) > div > div > ul > li")
        items_specs = response.css("article.-pvm:nth-child(2) > div > div > ul > li")
        
        for item in items_features:
            w = item.css("::text").get()
            prod_features.append(w)
        
        for item in items_specs:
            w = item.css("::text").get()
            prod_specs.append(w)
            
        prod_spec = {head_features:prod_features, head_specs:prod_specs}
        
        total_stars = f"{response.css("div.-fw-nw > div > div > div > span::text").get()}/5"
        total_reviews = response.css("div.-fw-nw > div > div > p::text").get()
        s_reviews = response.xpath("//div[@class='cola -phxl -df -d-co']/article[@class='-pvm -hr _bet']")
        s_r_data = []
        for review in s_reviews:
            star = review.css("div::text").get()
            head = review.css("h3::text").get()
            text = review.css("p::text").get()
            date = review.xpath("//div[@class='-df -j-bet -i-ctr -gy5']/div[@class='-pvm']/span[1]/text()").get()
            username = review.xpath("//div[@class='-df -j-bet -i-ctr -gy5']/div[@class='-pvm']/span[2]/text()").get()
            s_r_data.append({"stars":star, "heading":head, "text":text, "date":date, "username":username})

        reviews = {"total_stars":total_stars, "total_reviews":total_reviews, "user_reviews": s_r_data}
        
        brand = response.meta["brand"]
        if not brand:
            brand = response.css("div.-phm > div.-pvxs > a._more::text").get()
                     
        yield {
            "product_url": response.meta["url"],
            "product_name": response.meta["name"],
            "product_sku": response.meta["sku"],
            "product_brand": brand,
            "product_category": response.meta["category"],
            "product_image": response.meta["image"],
            "product_price": d_price,
            "product_price_text": price,
            "product_currency": "₦",
            "product_details": prod_detail,
            "product_specifications": prod_spec,
            "product_reviews": reviews,
            "score": 0
        }



