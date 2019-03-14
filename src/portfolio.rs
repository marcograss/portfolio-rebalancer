use serde_json::{Result};
use serde::{Deserialize};
use std::fs;

#[derive(Debug, Clone)]
#[derive(Deserialize)]
pub struct Asset {
	pub name: String,
	pub price: f32,
	pub count: u32,
	pub alloc: f32,
	#[serde(skip_deserializing)]
	pub value: f32,
}

#[derive(Debug, Clone)]
#[derive(Deserialize)]
pub struct Portfolio {
	pub assets: Vec<Asset>,
	#[serde(skip_deserializing)]
	pub value: f32,
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
		let mut sum: f32 = 0.0;
		for a in &self.assets {
			sum += a.alloc;
		}
		return sum == 100.0;
	}
	fn calculate_asset_values(&mut self) {
		for a in &mut self.assets {
			a.value = a.price * (a.count as f32);
			self.value += a.value;
		}
	}

	fn recalc_allocation(&mut self) {
		for a in &mut self.assets {
			a.alloc = (a.value/self.value)*100.0;
		}
	}

	pub fn rebalance(&self) -> Portfolio {
		let mut target_portfolio = self.clone();
		for a in &mut target_portfolio.assets {
			a.count = ((target_portfolio.value*a.alloc/100.0)/a.price) as u32;
			a.value = a.price * (a.count as f32);
		}
		target_portfolio.value = 0.0;
		for a in &target_portfolio.assets {
			target_portfolio.value += a.value;
		}
		// add leftover to currency
		if self.value > target_portfolio.value {
			for a in &mut target_portfolio.assets {
				// TODO improve this to make it generic
				if a.name == "USD" {
					a.count = (self.value - target_portfolio.value) as u32;
					a.value = a.price * (a.count as f32);
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
			let diff: i32 = b.count as i32 - a.count as i32;
			match diff {
				d if d == 0 => {
					// Nothing
				},
				d if d > 0 => {
					ret.push(Action{buysell: BuySell::Buy, amount: d as u32, name :a.name.clone()})
				},
				d if d < 0 => {
					ret.push(Action{buysell: BuySell::Sell, amount: ((-d) as u32), name :a.name.clone()})
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
			display_data.push((&a.name, a.alloc as u64));
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
	let v: Result<Portfolio> = serde_json::from_str(&data);
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
