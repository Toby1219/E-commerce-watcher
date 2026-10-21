use serde::{Serialize, Deserialize};
use validator::Validate;
use rust_decimal::Decimal;
use std::str::FromStr;
use utoipa::{ToSchema, IntoParams};

use entities::products::Column;

use crate::schemas::response_sch::{ApiError};


#[derive(Deserialize, Validate, ToSchema, IntoParams)]
pub struct CreateUser{
    /// User Email
    #[param(example="example@gmail.com")]
    #[validate(email)]
    pub email: String,
    /// Username minimun 3 
    #[param(example="user")]
    #[validate(length(min=3))]
    pub username: String,
    /// User passaword minimum 8
    #[validate(length(min=8))]
    pub password_text: String,
}

#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
pub struct UpdateUser{
    /// Update user Email
    #[param(example="example@gmail.com")]
    pub email: Option<String>,
    /// Update username
    pub username: Option<String>,
    // pub password_text: Option<String>
}

#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
pub struct LoginUser{
    /// User Email
    #[param(example="example@gmail.com")]
    pub email: String,
    /// User Password
    pub password_text: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct RefreshTokenRequest{
    /// Enter a valid refresh token string
    pub refresh_token: Option<String>
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all="lowercase")]
pub enum SortValues {
    ID,
    PRICE,
    CreatedAt,
}

impl SortValues{
    pub fn to_column(&self)-> Column{
        match self {
            SortValues::ID => Column::Id,
            SortValues::PRICE => Column::ProductPrice,
            SortValues::CreatedAt => Column::CreatedAt,
        }
    }

}

#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
pub struct ProductQuery{
    /// p_n = product name
    #[param(example="laptop")]
    pub p_n: Option<String>,
    
    /// s_t = sort type
    #[param(example="id")]
    pub s_t: Option<SortValues>,

    /// p = page
    #[param(example=1)]
    pub p: Option<u64>,

    /// p_p = per page
    #[param(example=5)]     
    pub p_p: Option<u64>,
}

#[derive(Deserialize, Serialize, ToSchema, IntoParams)]
pub struct ProductIdQuery{
    /// n = product name
    #[param(example="laptop")]
    pub n: u64
}

#[derive(Deserialize, Debug, ToSchema, IntoParams)]
pub struct BHOQuery{
    /// Product ID number
    #[param(example=1)]
    pub product_id:Option<u64>,

    /// History phrase (only used in history routes not bookmark or any other route)
    #[param(example=1)]
    pub phrase:Option<String>
}

#[derive(Deserialize, Debug, Clone, Copy, ToSchema)]
pub enum Converstions {
    OthersToBase,
    OthersToOthers,
}

impl FromStr for Converstions {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OTB" => Ok(Self::OthersToBase),
            "OTO" => Ok(Self::OthersToOthers),
            other => Err(ApiError::NotFound(format!("unknown conversion type: {other}"))),
        }
    }

}

impl Converstions {
    pub fn convert(self, 
        ammount:Decimal, 
        rate_one:Decimal, 
        rate_two:Option<Decimal>
    )->Result<Decimal, ApiError> {
        match self {
            Self::OthersToBase => Ok(ammount / rate_one),
            Self::OthersToOthers => {
                let rate_two = rate_two
                    .ok_or_else(|| ApiError::NotFound("currency_two is required for OTO".to_string()))?;
                Ok(ammount * rate_two / rate_one)   
            }
        }
    }
}

#[derive(Deserialize, Debug, IntoParams)]
pub struct CurrencyQuery{
    /// Money you want to convert
    #[param(example=1000, value_type = f64)]
    pub ammount: Decimal,

    /// Other currency 
    #[param(example="NGN")]
    pub from:String,

    /// Base currency (Defualt: USD)
    #[param(example="USD")]
    pub to:Option<String>,
    
    /// Convertion type we have two types
    /// Others To Base (Defult: USD) (OTB)
    /// Others To Others (OTO)
    #[param(example="OTB")]
    pub converstion_type:String
}


#[derive(Deserialize)]
pub struct NewItem { pub content: String }

#[derive(Deserialize, IntoParams)]
pub struct SearchReq {
    /// sematic word to be searched
    #[param(example = "Hp laptop")]
    pub query: String,
    /// limit to the number of product that will be return
    #[param(example = 5, minimum=1)]
    pub limit : Option<i64>
}

