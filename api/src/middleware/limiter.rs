use actix_governor::{GovernorConfigBuilder};
use crate::utils::constants::DefaultGovernorConfig;

pub struct RoutesLimiter{
    pub view_route: DefaultGovernorConfig,
    pub auth_route: DefaultGovernorConfig,
}

impl RoutesLimiter {
    pub fn new()->Self{
        let views_rate_limiter  = GovernorConfigBuilder::default()
            .requests_per_second(5)
            .burst_size(7)
            .finish()
            .unwrap();

        let auth_rate_limiter = GovernorConfigBuilder::default()
            .requests_per_second(5)
            .burst_size(7)
            .finish()
            .unwrap();

        Self { 
            view_route: views_rate_limiter, 
            auth_route: auth_rate_limiter,
        }
    }
}

