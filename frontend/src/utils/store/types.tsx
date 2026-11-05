type UserReview = {
    date: string,
    heading: string,
    stars: string,
    text: string,
    username: string,
}

type ProductDetail = {
    data: object
}

type ProductDetails = {
    details: ProductDetail[],
    summary: string,
}

type ProductReviews = {
    total_reviews: string,
    total_stars: string,
    user_reviews: UserReview[]
}

type ProductSpecifications = {
    Key_features: {
        "Key Features": string[];        
        "What's in the box": string[];
    },

}

export type Products = {
    id: number,
    product_url: string,
    product_name: string,
    product_price: string,
    product_brand_name: string,
    product_category: string,
    product_image: string,
    product_sku: string,
    product_details: ProductDetails,
    product_reviews: ProductReviews,
    product_specifications: ProductSpecifications,
    score: number,
    created_at: string

}

type SearchProduct = {
    created_at: string,
    phrase: string
}

export type Bookmarks = {
    total: number,
    products: Products[]
}

export type History = {
    total: number,
    search: SearchProduct[]
}

export interface CurrentUser{
    id:number,
    username:string,
    email:string,
    orders_count:number,
    date_joined:string,
}

// Product page types

export interface ProductReview {
  id: number;
  author: string;
  rating: number;
  date: string;
  heading: string;
  text: string;
}

export interface Product {
  id: number;
  name: string;
  brand: string;
  category: string;                          // raw "A/B/C" path, split into a breadcrumb on the page
  sku: string;
  price: number;
  rating: number;                            // 0 = unrated
  reviewCount: number;
  sizes: string[];
  image_url: string;
  originalUrl: string;
  description: { summary: string; details: string[] };
  specifications: Record<string, string[]>;  // e.g. "Key Features": [...], "What's in the box": [...]
  reviews: ProductReview[];
}
