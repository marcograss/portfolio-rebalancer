use serde_json::{Result};
use serde::{Deserialize};
use std::fs;
use rust_decimal_macros::*;
use rust_decimal::Decimal;
use num_traits::cast::ToPrimitive;
use json_comments::StripComments;
use std::io::Read;

#[derive(Debug, Clone)]
#[derive(Deserialize)]
pub struct Asset {
	pub name: String,
	pub price: Decimal,
	pub count: Decimal,
	pub alloc: Decimal,
	#[serde(skip_deserializing)]
	pub value: Decimal,
}

#[derive(Debug, Clone)]
#[derive(Deserialize)]
pub struct Portfolio {
	pub assets: Vec<Asset>,
	#[serde(default)]
	pub donotsell: bool,
	#[serde(skip_deserializing)]
	pub value: Decimal,
	#[serde(skip_deserializing)]
	pub currency: Option<Asset>,
}

#[derive(Debug, Clone)]
#[derive(Deserialize)]
pub enum BuySell {
	Buy,
	Sell,
}

#[derive(Debug, Clone)]
#[derive(Deserialize)]
pub struct Action {
	pub buysell: BuySell,
	pub amount: u32,
	pub name: String,
}

static CURRENCY: &str = "USD";

impl Portfolio {

	fn get_allocation_sum(&self) -> Decimal {
		let mut sum: Decimal = dec!(0.0);
		for a in &self.assets {
			sum += a.alloc;
		}
		return sum
	}

	fn is_target_allocation_sane(&self) -> bool {
		return self.get_allocation_sum() == dec!(100.0);
	}

	fn get_currency(&self) -> Option<Asset> {
		for a in &self.assets {
			if a.name == CURRENCY {
				return Some(a.clone());
			}
		}
		return None;
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
			a.alloc = (a.value/self.value)*dec!(100.0);
		}
	}

	pub fn rebalance(&self) -> Portfolio {
		let mut target_portfolio = self.clone();
		for a in &mut target_portfolio.assets {
			a.count = (target_portfolio.value*a.alloc/dec!(100.0))/a.price;
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

	pub fn add_without_selling(&self) -> Portfolio {
		let mut target_portfolio = self.clone();
		let currency = self.get_currency().unwrap();
		for a in &mut target_portfolio.assets {
			a.count = a.count + ((currency.value*a.alloc/dec!(100.0))/a.price);
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
		target_portfolio
	}

	pub fn get_actions(&self, target_portfolio: &Portfolio) -> Vec<Action> {
		let mut ret = Vec::new();
		for i in 0..self.assets.len() {
			let a = &self.assets[i];
			let b = &target_portfolio.assets[i];
			assert!(a.name == b.name);
			let diff: Decimal = b.count- a.count;
			match diff {
				d if d == dec!(0) => {
					// Nothing
				},
				d if d > dec!(0) => {
					ret.push(Action{buysell: BuySell::Buy, amount: d.to_u32().expect(&format!("cannot format {:?}", a)), name :a.name.clone()})
				},
				d if d < dec!(0) => {
					ret.push(Action{buysell: BuySell::Sell, amount: (-d.to_i32().expect(&format!("cannot format {:?}", a))) as u32, name :a.name.clone()})
				},
				_ => {},
			}
		}
		ret
	}

	pub fn get_display_data(&mut self) -> Vec<(&str, u64)> {
		self.recalc_allocation();
		let mut display_data: Vec<(&str, u64)> = Vec::new();
		for a in &self.assets {
			display_data.push((&a.name, a.alloc.to_u32().expect(&format!("cannot display {:?}", a)) as u64));
		}
		return display_data;
	}
}

pub fn load_portfolio(port_file: &str) -> std::result::Result<Portfolio, String>{
	let data = fs::read_to_string(port_file);
	if data.is_err() {
		return Err("Something went wrong reading the portfolio file".to_string());
	}
	let data = data.unwrap();
	let mut stripped = String::new();
	StripComments::new(data.as_bytes()).read_to_string(&mut stripped).unwrap();
	let v: Result<Portfolio> = serde_json::from_str(&stripped);
	match v {
		Ok(mut p) => {
			if !p.is_target_allocation_sane() {
				return Err(format!("Your portfolio target allocation sum is not 100%, it's {:?}%", p.get_allocation_sum()));
			}
			if !p.has_currency() {
				return Err(format!("Your portfolio doesn't have a {} asset", CURRENCY))
			}
			p.calculate_asset_values();
			return Ok(p);
		},
		Err(e) => {
			return Err(format!("Error parsing the portfolio json {:?}", e));
		},
	}
}
