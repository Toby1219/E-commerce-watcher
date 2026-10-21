use actix_web::cookie::{Cookie, SameSite, time::Duration as CookieDuration};
use actix_governor::{GovernorConfig, PeerIpKeyExtractor};
use actix_governor::governor::middleware::NoOpMiddleware;
use actix_governor::governor::clock::QuantaInstant;


pub fn set_cookies(access_token:String, refresh_token:String)->Vec<Cookie<'static>>{
    let access_token: Cookie<'_> = Cookie::build("access_token", access_token)
        .path("/")
        .http_only(true)
        .same_site(SameSite::None)
        .secure(true)
        .max_age(CookieDuration::days(1))
        .finish();

    let refresh_token: Cookie<'_> = Cookie::build("refresh_token", refresh_token)
        .path("/")
        .http_only(true)
        .same_site(SameSite::None)
        .secure(true)
        .max_age(CookieDuration::days(1))
        .finish();
    vec![access_token, refresh_token]
}

pub type DefaultGovernorConfig = GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware<QuantaInstant>>;