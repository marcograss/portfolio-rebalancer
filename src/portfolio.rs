use anyhow::anyhow;
use json_comments::StripComments;
use num_traits::cast::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use serde_json::Result;
use std::fs;
use std::io::Read;

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub price: Decimal,
    pub count: Decimal,
    pub alloc: Decimal,
    #[serde(skip_deserializing)]
    pub value: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Portfolio {
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub donotsell: bool,
    #[serde(skip_deserializing)]
    pub value: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub enum BuySell {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Action {
    pub buysell: BuySell,
    pub amount: u32,
    pub name: String,
    pub transaction_value: Decimal,
}

static CURRENCY: &str = "USD";

impl Portfolio {
    fn get_allocation_sum(&self) -> Decimal {
        let mut sum: Decimal = dec!(0.0);
        for a in &self.assets {
            sum += a.alloc;
        }
        sum
    }

    fn is_target_allocation_sane(&self) -> bool {
        self.get_allocation_sum() == dec!(100.0)
    }

    fn get_currency(&self) -> Option<Asset> {
        for a in &self.assets {
            if a.name == CURRENCY {
                return Some(a.clone());
            }
        }
        None
    }

    fn has_currency(&self) -> bool {
        self.get_currency().is_some()
    }

    fn calculate_asset_values(&mut self) {
        for a in &mut self.assets {
            a.value = a.price * (a.count);
            self.value += a.value;
        }
    }

    fn recalc_allocation(&mut self) {
        for a in &mut self.assets {
            a.alloc = (a.value / self.value) * dec!(100.0);
        }
    }

    pub fn rebalance(&self) -> Self {
        let mut target_portfolio = self.clone();
        for a in &mut target_portfolio.assets {
            a.count = (target_portfolio.value * a.alloc / dec!(100.0)) / a.price;
            a.value = a.price * a.count;
        }
        target_portfolio.value = dec!(0.0);
        for a in &target_portfolio.assets {
            target_portfolio.value += a.value;
        }
        // add leftover to currency
        if self.value > target_portfolio.value {
            for a in &mut target_portfolio.assets {
                // TODO improve this to make it generic
                if a.name == CURRENCY {
                    a.count = self.value - target_portfolio.value;
                    a.value = a.price * (a.count);
                    target_portfolio.value += a.value;
                    break;
                }
            }
        }
        target_portfolio.recalc_allocation();
        target_portfolio
    }

    pub fn add_without_selling(&self) -> anyhow::Result<Self> {
        let mut target_portfolio = self.clone();
        let currency = self
            .get_currency()
            .ok_or_else(|| anyhow!("cannot get currency"))?;
        for a in &mut target_portfolio.assets {
            a.count += (currency.value * a.alloc / dec!(100.0)) / a.price;
            a.value = a.price * a.count;
            if a.name == CURRENCY {
                a.count = dec!(0.0);
                a.value = dec!(0.0);
            }
        }
        target_portfolio.value = dec!(0.0);
        for a in &target_portfolio.assets {
            target_portfolio.value += a.value;
        }
        target_portfolio.recalc_allocation();
        Ok(target_portfolio)
    }

    pub fn get_actions(&self, target_portfolio: &Self) -> anyhow::Result<Vec<Action>> {
        let mut ret = Vec::new();
        for i in 0..self.assets.len() {
            let a = &self.assets[i];
            let b = &target_portfolio.assets[i];
            assert!(a.name == b.name);
            let diff: Decimal = b.count - a.count;
            let transaction_value: Decimal = (diff * a.price).abs();
            match diff {
                d if d == dec!(0) => {
                    // Nothing
                }
                d if d > dec!(0) => ret.push(Action {
                    buysell: BuySell::Buy,
                    amount: d.to_u32().ok_or_else(|| anyhow!("cannot format {a:?}"))?,
                    name: a.name.clone(),
                    transaction_value,
                }),
                d if d < dec!(0) => ret.push(Action {
                    buysell: BuySell::Sell,
                    amount: u32::try_from(
                        -d.to_i32().ok_or_else(|| anyhow!("cannot format {a:?}"))?,
                    )?,
                    name: a.name.clone(),
                    transaction_value,
                }),
                _ => {}
            }
        }
        Ok(ret)
    }

    pub fn get_display_data(&mut self) -> anyhow::Result<Vec<(&str, u64)>> {
        self.recalc_allocation();
        let mut display_data: Vec<(&str, u64)> = Vec::new();
        for a in &self.assets {
            display_data.push((
                &a.name,
                u64::from(
                    a.alloc
                        .to_u32()
                        .ok_or_else(|| anyhow!("cannot format {a:?}"))?,
                ),
            ));
        }
        Ok(display_data)
    }
}

pub fn load_portfolio_from_file(port_file: &str) -> anyhow::Result<Portfolio> {
    let data = fs::read_to_string(port_file)?;
    let mut stripped = String::new();
    StripComments::new(data.as_bytes()).read_to_string(&mut stripped)?;
    let v: Result<Portfolio> = serde_json::from_str(&stripped);
    match v {
        Ok(mut p) => {
            if !p.is_target_allocation_sane() {
                return Err(anyhow!(
                    "Your portfolio target allocation sum is not 100%, it's {:?}%",
                    p.get_allocation_sum()
                ));
            }
            if !p.has_currency() {
                return Err(anyhow!("Your portfolio doesn't have a {CURRENCY} asset"));
            }
            p.calculate_asset_values();
            Ok(p)
        }
        Err(e) => Err(anyhow!("Error parsing the portfolio json {e:?}")),
    }
}
