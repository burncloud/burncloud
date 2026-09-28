#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UsagePeriod {
    #[default]
    SevenDays,
    ThirtyDays,
    NinetyDays,
}

impl UsagePeriod {
    pub const ALL: [Self; 3] = [Self::SevenDays, Self::ThirtyDays, Self::NinetyDays];
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UsageState {
    pub period: UsagePeriod,
}

impl UsageState {
    pub fn set_period(&mut self, period: UsagePeriod) {
        self.period = period;
    }
}
