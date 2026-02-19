pub mod ddos_protection;
pub mod ip_filter;
pub mod rate_limit;
pub mod size_limit;
pub mod stack;

// Re-export all protection features for convenience
pub use ddos_protection::{
    DdosError, DdosMetrics, DdosProtectionConfig, DdosProtectionService, IpMetrics,
    ddos_protection_middleware,
};
pub use ip_filter::{
    DefaultPolicy, IpFilterConfig, IpFilterError, IpFilterService, ip_filter_middleware,
};
pub use rate_limit::{
    RateLimitConfig, RateLimitError, RateLimitService, RateLimitVaryBy, rate_limit_middleware,
};
pub use size_limit::{
    SizeLimitConfig, SizeLimitError, SizeLimitService, content_length_middleware,
    size_limit_middleware,
};
pub use stack::{ProtectionStack, ProtectionStackBuilder, SizeLimitMode};

// Re-export presets (each module has its own presets submodule)
pub use ddos_protection::presets as ddos_presets;
pub use ip_filter::presets as ip_filter_presets;
pub use rate_limit::presets as rate_limit_presets;
pub use size_limit::presets as size_limit_presets;
