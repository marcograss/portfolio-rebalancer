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
	#[serde(skip_deserializing)]
	pub value: Decimal,
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

impl Portfolio {
	fn is_target_allocation_sane(&self) -> bool {
		let mut sum: Decimal = dec!(0.0);
		for a in &self.assets {
			sum += a.alloc;
		}
		return sum == dec!(100.0);
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
				if a.name == "USD" {
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
				return Err("Your portfolio target allocation sum is not 100%".to_string());
			}
			p.calculate_asset_values();
			return Ok(p);
		},
		Err(e) => {
			return Err(format!("Error parsing the portfolio json {:?}", e));
		},
	}
}
