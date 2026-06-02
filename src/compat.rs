use orion_error::UnifiedReason;

pub use orion_error::conversion::ConvErr as ErrorConv;

pub trait UvsFrom: Sized + From<UnifiedReason> {
    fn from_conf() -> Self {
        Self::from(UnifiedReason::core_conf())
    }

    fn from_validation() -> Self {
        Self::from(UnifiedReason::validation_error())
    }

    fn from_rule() -> Self {
        Self::from(UnifiedReason::rule_error())
    }

    fn from_res() -> Self {
        Self::from(UnifiedReason::resource_error())
    }

    fn from_biz() -> Self {
        Self::from(UnifiedReason::business_error())
    }

    fn from_logic() -> Self {
        Self::from(UnifiedReason::logic_error())
    }
}

impl<T> UvsFrom for T where T: From<UnifiedReason> {}
