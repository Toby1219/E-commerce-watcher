from tortoise import fields 
from tortoise.models import Model 


class Product(Model):
    id = fields.BigIntField(primary_key=True)
    product_url = fields.CharField(max_length=2048)  
    product_name = fields.CharField(max_length=512) 
    product_price = fields.CharField(max_length=64) 
    product_brand_name = fields.CharField(max_length=255, null=True)
    product_category = fields.CharField(max_length=255, null=True)
    product_image = fields.CharField(max_length=2048, null=True)  
    product_sku = fields.CharField(max_length=128, null=True)
    product_details = fields.JSONField()
    product_reviews = fields.JSONField()
    product_specifications = fields.JSONField()
    score = fields.BigIntField()
    created_at = fields.DatetimeField(auto_now_add=True)

    class Meta:
        table = "products"

    def __str__(self):
        return f"<Product {self.id}: {self.product_name}>"


class Currencies(Model):
    id = fields.BigIntegerField(primary_key=True)
    base_currency = fields.CharField(max_length=5, null=True)
    currency = fields.CharField(max_length=5, null=True)
    rates = fields.DecimalField(max_digits=10, decimal_places=4)
    time_last_update_utc = fields.DateTimeField()
    time_next_update_utc = fields.DateTimeField()
    
    class Meta:
        table = "currencies"
    
    def __str__(self):
        return f"Curency {self.id}: {self.currency}"
    